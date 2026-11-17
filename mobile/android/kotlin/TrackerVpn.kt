package dev.formicaria.notes

import android.content.Intent
import android.net.ConnectivityManager
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.system.OsConstants
import org.json.JSONArray
import org.json.JSONObject
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

class TrackerVpn : VpnService() {
  private var tun: ParcelFileDescriptor? = null
  @Volatile private var running = false
  private val pool = Executors.newFixedThreadPool(4)

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    if (intent?.action == STOP) { close(); stopSelf(); return START_NOT_STICKY }
    if (running) return START_STICKY
    upstream = resolverOfNetwork() ?: "9.9.9.9"
    val b = Builder().setSession("formicaria: trackers refused")
      .addAddress(OWN, 32).addDnsServer(DNS).addRoute(DNS, 32).setBlocking(true)
    // Only lookups enter: formicaria itself (its pages block on their own) and apps let through stay out.
    for (pkg in allowed + packageName) try { b.addDisallowedApplication(pkg) } catch (e: Exception) { }
    tun = b.establish() ?: return START_NOT_STICKY
    running = true
    on = true
    Thread { loop() }.start()
    return START_STICKY
  }

  private fun resolverOfNetwork(): String? {
    val cm = getSystemService(ConnectivityManager::class.java) ?: return null
    val lp = cm.getLinkProperties(cm.activeNetwork) ?: return null
    return lp.dnsServers.firstOrNull { it.address.size == 4 }?.hostAddress
  }

  private fun loop() {
    val fd = tun ?: return
    val input = FileInputStream(fd.fileDescriptor)
    val output = FileOutputStream(fd.fileDescriptor)
    val buf = ByteArray(32767)
    while (running) {
      val n = try { input.read(buf) } catch (e: Exception) { break }
      if (n <= 28) continue
      val pkt = buf.copyOf(n)
      // IPv4, UDP, to port 53.
      if ((pkt[0].toInt() shr 4) != 4 || pkt[9].toInt() != 17) continue
      val ihl = (pkt[0].toInt() and 0x0f) * 4
      if (n < ihl + 8 + 12 || port(pkt, ihl + 2) != 53) continue
      pool.execute { answer(pkt, ihl, output) }
    }
    close()
  }

  private fun port(p: ByteArray, at: Int) = ((p[at].toInt() and 0xff) shl 8) or (p[at + 1].toInt() and 0xff)

  private fun answer(pkt: ByteArray, ihl: Int, output: FileOutputStream) {
    val query = pkt.copyOfRange(ihl + 8, pkt.size)
    val name = qname(query) ?: return
    val reply = if (try { Blocker.hostBlocked(name) } catch (e: Throwable) { false }) {
      count(pkt, ihl, name)
      refuse(query)
    } else forward(query) ?: return
    synchronized(output) { try { output.write(wrap(pkt, ihl, reply)) } catch (e: Exception) { } }
  }

  /** The name asked for (the first question). */
  private fun qname(q: ByteArray): String? {
    var i = 12
    val parts = ArrayList<String>()
    while (i < q.size) {
      val len = q[i].toInt() and 0xff
      if (len == 0) return parts.joinToString(".").lowercase()
      if (len > 63 || i + 1 + len > q.size) return null
      parts += String(q, i + 1, len, Charsets.US_ASCII)
      i += 1 + len
    }
    return null
  }

  /** "No such address": the query back, as an answer, with no records. */
  private fun refuse(q: ByteArray): ByteArray {
    val r = q.copyOf()
    r[2] = (0x80 or (q[2].toInt() and 0x79)).toByte() // QR, keep opcode and RD
    r[3] = 0x83.toByte() // RA, NXDOMAIN
    for (k in 6..11) r[k] = 0
    return r
  }

  private fun forward(q: ByteArray): ByteArray? = try {
    DatagramSocket().use { s ->
      protect(s)
      s.soTimeout = 5000
      s.send(DatagramPacket(q, q.size, InetAddress.getByName(upstream), 53))
      val back = ByteArray(4096)
      val p = DatagramPacket(back, back.size)
      s.receive(p)
      back.copyOf(p.length)
    }
  } catch (e: Exception) { null }

  /** The answer as the packet the asking app expects: addresses and ports swapped. */
  private fun wrap(req: ByteArray, ihl: Int, dns: ByteArray): ByteArray {
    val total = 20 + 8 + dns.size
    val out = ByteArray(total)
    out[0] = 0x45; out[2] = (total shr 8).toByte(); out[3] = total.toByte()
    out[6] = 0x40; out[8] = 64; out[9] = 17
    System.arraycopy(req, 16, out, 12, 4) // from: where it was sent
    System.arraycopy(req, 12, out, 16, 4) // to: who sent it
    var sum = 0
    for (k in 0 until 20 step 2) sum += ((out[k].toInt() and 0xff) shl 8) or (out[k + 1].toInt() and 0xff)
    while (sum shr 16 != 0) sum = (sum and 0xffff) + (sum shr 16)
    val c = sum.inv() and 0xffff
    out[10] = (c shr 8).toByte(); out[11] = c.toByte()
    System.arraycopy(req, ihl + 2, out, 20, 2) // source port: 53
    System.arraycopy(req, ihl, out, 22, 2)     // destination port: theirs
    val ul = 8 + dns.size
    out[24] = (ul shr 8).toByte(); out[25] = ul.toByte() // checksum 0: none (IPv4)
    System.arraycopy(dns, 0, out, 28, dns.size)
    return out
  }

  private fun count(pkt: ByteArray, ihl: Int, name: String) {
    var app = "?"
    if (Build.VERSION.SDK_INT >= 29) try {
      val cm = getSystemService(ConnectivityManager::class.java)
      val src = InetSocketAddress(InetAddress.getByAddress(pkt.copyOfRange(12, 16)), port(pkt, ihl))
      val dst = InetSocketAddress(InetAddress.getByAddress(pkt.copyOfRange(16, 20)), 53)
      val uid = cm.getConnectionOwnerUid(OsConstants.IPPROTO_UDP, src, dst)
      app = packageManager.getPackagesForUid(uid)?.firstOrNull() ?: "?"
    } catch (e: Exception) { }
    val per = seen.getOrPut(app) { ConcurrentHashMap() }
    per[name] = (per[name] ?: 0) + 1
  }

  private fun close() {
    running = false
    on = false
    try { tun?.close() } catch (e: Exception) { }
    tun = null
  }

  override fun onRevoke() { close(); super.onRevoke() }
  override fun onDestroy() { close(); pool.shutdownNow(); super.onDestroy() }

  companion object {
    const val STOP = "dev.formicaria.notes.VPN_STOP"
    private const val OWN = "10.111.222.1"
    private const val DNS = "10.111.222.2"
    @Volatile var on = false
    @Volatile var upstream = "9.9.9.9"
    /** Apps let through (the person's choice, kept by the app). */
    @Volatile var allowed: Set<String> = emptySet()
    /** Which app asked for which tracker, how often, since the block started. */
    val seen = ConcurrentHashMap<String, ConcurrentHashMap<String, Int>>()

    fun state(labels: (String) -> String): String {
      val apps = JSONArray()
      for ((pkg, names) in seen.entries.sortedByDescending { e -> e.value.values.sum() }) {
        val top = JSONArray(names.entries.sortedByDescending { it.value }.take(20).map { JSONObject().put("host", it.key).put("n", it.value) })
        apps.put(JSONObject().put("pkg", pkg).put("label", labels(pkg)).put("n", names.values.sum()).put("hosts", top))
      }
      return JSONObject().put("on", on).put("apps", apps).put("allowed", JSONArray(allowed.toList())).toString()
    }
  }
}
