package dev.formicaria.notes

import android.content.Intent
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
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
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.URL
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

/**
 * **Trackers inside other apps, refused for the whole phone** (`decisions.md` 2026-11-17;
 * `privacy-plan-2026-11-19.md` stage 4). A local VPN that carries only address lookups: the phone's
 * DNS server is this service's own address, so each app's lookup arrives here, is answered "no such
 * name" when the name (or a name its answer leads through) is a tracker's (`Blocker.hostBlocked`,
 * the app lists of `fm-block::lists`), and otherwise is sent on: **encrypted** (DNS over HTTPS) to
 * the resolver chosen in Privacy (Quad9 by default), or to the network's own when that is chosen or
 * the encrypted one cannot be reached. Answers are kept for their time to live (at most an hour).
 * Nothing else passes through it; formicaria itself and the apps let through stay outside.
 */
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
    // IPv6 lookups too, on a network that offers only IPv6 resolvers.
    try { b.addAddress(OWN6, 128).addDnsServer(DNS6).addRoute(DNS6, 128) } catch (e: Exception) { }
    // Only lookups enter: formicaria itself (its pages block on their own) and apps let through stay out.
    for (pkg in allowed + packageName) try { b.addDisallowedApplication(pkg) } catch (e: Exception) { }
    tun = b.establish() ?: return START_NOT_STICKY
    running = true
    on = true
    cache.clear()
    Thread { loop() }.start()
    return START_STICKY
  }

  private fun underlying(): android.net.LinkProperties? {
    val cm = getSystemService(ConnectivityManager::class.java) ?: return null
    val net = cm.allNetworks.firstOrNull {
      val c = cm.getNetworkCapabilities(it)
      c != null && c.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) && !c.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
    } ?: cm.activeNetwork ?: return null
    return cm.getLinkProperties(net)
  }

  private fun resolverOfNetwork(): String? =
    underlying()?.dnsServers?.firstOrNull { it.address.size == 4 }?.hostAddress

  private fun loop() {
    val fd = tun ?: return
    val input = FileInputStream(fd.fileDescriptor)
    val output = FileOutputStream(fd.fileDescriptor)
    val buf = ByteArray(32767)
    while (running) {
      val n = try { input.read(buf) } catch (e: Exception) { break }
      if (n <= 28) continue
      val pkt = buf.copyOf(n)
      val v = pkt[0].toInt() shr 4
      // IPv4 or IPv6, UDP, to port 53.
      val ihl = when {
        v == 4 && pkt[9].toInt() == 17 -> (pkt[0].toInt() and 0x0f) * 4
        v == 6 && n > 48 && pkt[6].toInt() == 17 -> 40
        else -> continue
      }
      if (n < ihl + 8 + 12 || port(pkt, ihl + 2) != 53) continue
      pool.execute { answer(pkt, ihl, output) }
    }
    close()
  }

  private fun port(p: ByteArray, at: Int) = ((p[at].toInt() and 0xff) shl 8) or (p[at + 1].toInt() and 0xff)

  private fun answer(pkt: ByteArray, ihl: Int, output: FileOutputStream) {
    val query = pkt.copyOfRange(ihl + 8, pkt.size)
    val name = try { Blocker.dnsName(query) } catch (e: Throwable) { "" }
    if (name.isEmpty()) return
    val reply = if (refused(name)) {
      count(pkt, ihl, name)
      Blocker.dnsRefuse(query)
    } else {
      val key = name + "/" + qtype(query)
      val kept = cache[key]?.takeIf { it.second > System.currentTimeMillis() }?.first
      val got = if (kept != null) Blocker.dnsWithId(kept, query) else resolve(query) ?: return
      if (kept == null) {
        // A tracker behind another name (its CNAME): refused like the tracker itself.
        if (try { Blocker.dnsCloaked(got) } catch (e: Throwable) { false }) {
          count(pkt, ihl, name)
          val r = Blocker.dnsRefuse(query)
          synchronized(output) { try { output.write(wrap(pkt, ihl, r)) } catch (e: Exception) { } }
          return
        }
        val ttl = try { Blocker.dnsTtl(got) } catch (e: Throwable) { -1 }
        if (ttl > 0) {
          if (cache.size > 2000) cache.clear()
          cache[key] = got to System.currentTimeMillis() + ttl * 1000L
        }
      }
      got
    }
    synchronized(output) { try { output.write(wrap(pkt, ihl, reply)) } catch (e: Exception) { } }
  }

  private fun refused(name: String): Boolean = try { Blocker.hostBlocked(name) } catch (e: Throwable) { false }

  private fun qtype(q: ByteArray): Int {
    var i = 12
    while (i < q.size && q[i].toInt() != 0) i += 1 + (q[i].toInt() and 0xff)
    return if (i + 2 < q.size) port(q, i + 1) else 0
  }

  /** The lookup sent on: encrypted to the chosen resolver, else (or failing that) the network's. */
  private fun resolve(q: ByteArray): ByteArray? {
    val doh = PagePrivacy.resolver
    if (doh.isNotEmpty()) overHttps(doh, q)?.let { return it }
    return plain(q)
  }

  private fun overHttps(url: String, q: ByteArray): ByteArray? = try {
    val c = URL(url).openConnection() as HttpURLConnection
    c.requestMethod = "POST"
    c.doOutput = true
    c.connectTimeout = 4000
    c.readTimeout = 4000
    c.setRequestProperty("Content-Type", "application/dns-message")
    c.setRequestProperty("Accept", "application/dns-message")
    c.outputStream.use { it.write(q) }
    if (c.responseCode != 200) null else c.inputStream.use { it.readBytes() }.takeIf { it.size >= 12 }
  } catch (e: Exception) { null }

  private fun plain(q: ByteArray): ByteArray? = try {
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
  private fun wrap(req: ByteArray, ihl: Int, dns: ByteArray): ByteArray =
    if ((req[0].toInt() shr 4) == 6) wrap6(req, dns) else wrap4(req, ihl, dns)

  private fun wrap4(req: ByteArray, ihl: Int, dns: ByteArray): ByteArray {
    val total = 20 + 8 + dns.size
    val out = ByteArray(total)
    out[0] = 0x45; out[2] = (total shr 8).toByte(); out[3] = total.toByte()
    out[6] = 0x40; out[8] = 64; out[9] = 17
    System.arraycopy(req, 16, out, 12, 4) // from: where it was sent
    System.arraycopy(req, 12, out, 16, 4) // to: who sent it
    val c = fold(sum(out, 0, 20)).inv() and 0xffff
    out[10] = (c shr 8).toByte(); out[11] = c.toByte()
    System.arraycopy(req, ihl + 2, out, 20, 2) // source port: 53
    System.arraycopy(req, ihl, out, 22, 2)     // destination port: theirs
    val ul = 8 + dns.size
    out[24] = (ul shr 8).toByte(); out[25] = ul.toByte() // checksum 0: none (IPv4)
    System.arraycopy(dns, 0, out, 28, dns.size)
    return out
  }

  /** IPv6: the UDP checksum is required, over the pseudo-header (addresses, length, protocol). */
  private fun wrap6(req: ByteArray, dns: ByteArray): ByteArray {
    val ul = 8 + dns.size
    val out = ByteArray(40 + ul)
    out[0] = 0x60
    out[4] = (ul shr 8).toByte(); out[5] = ul.toByte()
    out[6] = 17; out[7] = 64
    System.arraycopy(req, 24, out, 8, 16)  // from: where it was sent
    System.arraycopy(req, 8, out, 24, 16)  // to: who sent it
    System.arraycopy(req, 42, out, 40, 2)  // source port: 53
    System.arraycopy(req, 40, out, 42, 2)  // destination port: theirs
    out[44] = (ul shr 8).toByte(); out[45] = ul.toByte()
    System.arraycopy(dns, 0, out, 48, dns.size)
    val s = sum(out, 8, 32) + ul + 17 + sum(out, 40, ul)
    var c = fold(s).inv() and 0xffff
    if (c == 0) c = 0xffff
    out[46] = (c shr 8).toByte(); out[47] = c.toByte()
    return out
  }

  private fun sum(b: ByteArray, from: Int, len: Int): Int {
    var s = 0
    var i = from
    while (i + 1 < from + len) { s += ((b[i].toInt() and 0xff) shl 8) or (b[i + 1].toInt() and 0xff); i += 2 }
    if (i < from + len) s += (b[i].toInt() and 0xff) shl 8
    return s
  }

  private fun fold(x: Int): Int { var s = x; while (s shr 16 != 0) s = (s and 0xffff) + (s ushr 16); return s }

  private fun count(pkt: ByteArray, ihl: Int, name: String) {
    var app = "?"
    if (Build.VERSION.SDK_INT >= 29) try {
      val cm = getSystemService(ConnectivityManager::class.java)
      val v6 = (pkt[0].toInt() shr 4) == 6
      val src = InetSocketAddress(InetAddress.getByAddress(if (v6) pkt.copyOfRange(8, 24) else pkt.copyOfRange(12, 16)), port(pkt, ihl))
      val dst = InetSocketAddress(InetAddress.getByAddress(if (v6) pkt.copyOfRange(24, 40) else pkt.copyOfRange(16, 20)), 53)
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
    private const val OWN6 = "fd66:6f72:6d69::1"
    private const val DNS6 = "fd66:6f72:6d69::2"
    @Volatile var on = false
    @Volatile var upstream = "9.9.9.9"
    /** Apps let through (the person's choice, kept by the app). */
    @Volatile var allowed: Set<String> = emptySet()
    /** Which app asked for which tracker, how often, since the block started. */
    val seen = ConcurrentHashMap<String, ConcurrentHashMap<String, Int>>()
    /** Answers kept for their time to live: (name/type) → (answer, until). */
    private val cache = ConcurrentHashMap<String, Pair<ByteArray, Long>>()

    /** **Android's Private DNS set to a provider** (`privacy-plan-2026-11-19.md` 2.6): then other
     *  apps' lookups go encrypted to that provider and never reach this block. Its name, or null. */
    fun privateDns(ctx: android.content.Context): String? {
      if (Build.VERSION.SDK_INT < 28) return null
      return try {
        val cm = ctx.getSystemService(ConnectivityManager::class.java)
        cm.allNetworks.mapNotNull { n ->
          val c = cm.getNetworkCapabilities(n)
          if (c == null || c.hasTransport(NetworkCapabilities.TRANSPORT_VPN)) null else cm.getLinkProperties(n)?.privateDnsServerName
        }.firstOrNull()
      } catch (e: Exception) { null }
    }

    fun state(ctx: android.content.Context, labels: (String) -> String): String {
      val apps = JSONArray()
      for ((pkg, names) in seen.entries.sortedByDescending { e -> e.value.values.sum() }) {
        val top = JSONArray(names.entries.sortedByDescending { it.value }.take(20).map { JSONObject().put("host", it.key).put("n", it.value) })
        apps.put(JSONObject().put("pkg", pkg).put("label", labels(pkg)).put("n", names.values.sum()).put("hosts", top))
      }
      val ready = try { Blocker.appsReady() } catch (e: Throwable) { false }
      return JSONObject().put("on", on).put("apps", apps).put("allowed", JSONArray(allowed.toList()))
        .put("ready", ready).put("private_dns", privateDns(ctx) ?: JSONObject.NULL).toString()
    }
  }
}
