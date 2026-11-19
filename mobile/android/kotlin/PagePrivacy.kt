package dev.formicaria.notes

import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap

object PagePrivacy {
  @Volatile var trackers = true
  @Volatile var ads = false
  @Volatile var cookies = true
  @Volatile var cleanLinks = true
  @Volatile var gpc = true
  @Volatile var httpsOnly = true
  @Volatile var appTrackers = false
  /** *Stop apps going around the block*: public encrypted-DNS services refused. */
  @Volatile var around = false
  /** Where the app blocker sends the lookups it lets through (DNS over HTTPS; empty: the network's). */
  @Volatile var resolver = "https://dns.quad9.net/dns-query"
  @Volatile var fired = -1L
  @Volatile var sitesOff: List<String> = emptyList()
  /** The cookie pop-up script (`ui/page/consent.ts`), handed over by the app. */
  @Volatile var consentScript: String? = null
  /** The procedural hide rules' interpreter (`ui/page/procedural.ts`), handed over by the app. */
  @Volatile var proceduralScript: String? = null
  /** The fingerprinting defence (`ui/page/fingerprint.ts`), handed over by the app. */
  @Volatile var fingerprintScript: String? = null
  /** This session's secret for it: sites' readings stay the same within a session, differ between
   *  sites, and change when the fire button is pressed. Never sent anywhere. */
  @Volatile private var seed: String = java.util.UUID.randomUUID().toString()
  fun reseed() { seed = java.util.UUID.randomUUID().toString() }
  /** Sites the person chose to open without a secure connection, this session. */
  val insecureOk: MutableSet<String> = ConcurrentHashMap.newKeySet()

  /** Take the app's switches (`privacy.svelte.ts`). Answers whether the fire button was pressed
   *  since the last reading (never on the first). */
  fun update(json: String): Boolean {
    val o = JSONObject(json)
    trackers = o.optBoolean("trackers", true)
    ads = o.optBoolean("ads", false)
    cookies = o.optBoolean("cookies", true)
    cleanLinks = o.optBoolean("clean_links", true)
    gpc = o.optBoolean("gpc", true)
    httpsOnly = o.optBoolean("https_only", true)
    appTrackers = o.optBoolean("app_trackers", false)
    around = o.optBoolean("around", false)
    val dns = o.optString("dns", "quad9")
    val offered = o.optJSONArray("resolvers")
    resolver = (0 until (offered?.length() ?: 0)).map { offered!!.getJSONObject(it) }
      .firstOrNull { it.optString("key") == dns }?.optString("url")
      ?: if (dns == "network") "" else "https://dns.quad9.net/dns-query"
    val arr = o.optJSONArray("sites_off") ?: JSONArray()
    sitesOff = (0 until arr.length()).map { arr.optString(it).lowercase() }.filter { it.isNotEmpty() }
    val f = o.optLong("fired", 0)
    val pressed = fired >= 0 && f > fired
    fired = f
    return pressed
  }

  fun host(url: String?): String =
    (try { Uri.parse(url ?: "").host } catch (e: Exception) { null } ?: "").lowercase().removePrefix("www.")

  /** Every protection is off on this site (and its subdomains). */
  fun off(url: String?): Boolean {
    val h = host(url)
    return h.isNotEmpty() && sitesOff.any { h == it || h.endsWith(".$it") }
  }

  private val TRACKING = setOf(
    "fbclid", "gclid", "dclid", "gbraid", "wbraid", "msclkid", "yclid", "twclid", "ttclid", "igshid",
    "mc_cid", "mc_eid", "_hsenc", "_hsmi", "mkt_tok", "oly_anon_id", "oly_enc_id", "vero_id", "vero_conv",
    "rb_clickid", "s_cid", "wickedid", "__s", "ss_source", "ss_campaign_id", "srsltid", "epik"
  )
  private fun tracking(k: String) = k.lowercase().let { it.startsWith("utm_") || it in TRACKING }

  /** The address without its tracking parameters, or null when it had none: a tracker's redirect
   *  skipped and the lists' `$removeparam` rules (`Blocker.cleanLink`), then the names below. */
  fun clean(url: String): String? {
    if (!cleanLinks || off(url)) return null
    val listed = try { Blocker.cleanLink(url) } catch (e: Throwable) { "" }
    val start = if (listed.isNotEmpty()) listed else url
    val own = cleanOwn(start)
    return own ?: listed.ifEmpty { null }
  }

  private fun cleanOwn(url: String): String? {
    val u = try { Uri.parse(url) } catch (e: Exception) { return null }
    if (u.isOpaque || u.encodedQuery.isNullOrEmpty()) return null
    val names = u.queryParameterNames
    if (names.none { tracking(it) }) return null
    val kept = u.encodedQuery!!.split('&').filter { it.isNotEmpty() && !tracking(Uri.decode(it.substringBefore('='))) }
    return u.buildUpon().encodedQuery(if (kept.isEmpty()) null else kept.joinToString("&")).build().toString()
  }

  /** The `https` address for a plain `http` one, or null (already secure, local, or allowed). */
  fun upgrade(url: String): String? {
    if (!httpsOnly || !url.startsWith("http://") || off(url)) return null
    val h = host(url)
    if (h.isEmpty() || h == "localhost" || h in insecureOk || Regex("^[0-9.]+$|^\\[").containsMatchIn(h)) return null
    return "https://" + url.removePrefix("http://")
  }

  private fun guard(body: String): String {
    val list = JSONArray(sitesOff).toString()
    return "(function(){try{var off=$list;var h=location.hostname.toLowerCase().replace(/^www\\./,'');" +
      "if(off.some(function(s){return h===s||h.endsWith('.'+s)}))return;}catch(e){}\n$body\n})();"
  }

  /** The scripts a page gets at its start, as the switches stand. */
  fun startScripts(): List<String> {
    val out = ArrayList<String>()
    if (gpc) out += guard(
      "try{if(!('globalPrivacyControl' in navigator))Object.defineProperty(Navigator.prototype," +
        "'globalPrivacyControl',{get:function(){return true},configurable:true});}catch(e){}"
    )
    val consent = consentScript
    if (cookies && consent != null) out += guard(consent)
    // The lists' hide rules: the page's own, then the generic ones for the names it uses, again as
    // it changes (at most once a second).
    if (trackers || ads) out += guard(HIDE)
    val procedural = proceduralScript
    if ((trackers || ads) && procedural != null) out += guard(procedural)
    val fingerprint = fingerprintScript
    if (trackers && fingerprint != null) out += guard("window.__fmSeed=\"$seed\";\n$fingerprint")
    return out
  }

  private const val HIDE = """
    (function(){
      var c = window.__fmPagePrivacy; if (!c || !c.hide) return;
      var style = document.createElement('style'), seenC = {}, seenI = {}, css = '';
      function put(){ style.textContent = css; (document.head||document.documentElement).appendChild(style); }
      function names(root){
        var cl = [], id = [];
        var all = (root||document).querySelectorAll('[class],[id]');
        for (var i = 0; i < all.length && i < 20000; i++) {
          var e = all[i];
          if (e.id && !seenI[e.id]) { seenI[e.id] = 1; id.push(e.id); }
          var k = e.classList; if (k) for (var j = 0; j < k.length; j++) if (!seenC[k[j]]) { seenC[k[j]] = 1; cl.push(k[j]); }
        }
        if (cl.length || id.length) { var more = c.hideNames(location.href, cl.join(' '), id.join(' ')); if (more) { css += '\n' + more; put(); } }
      }
      function start(){
        try { css = c.hide(location.href) || ''; if (css) put(); names(); } catch (e) {}
        var t = 0;
        new MutationObserver(function(){ if (t) return; t = setTimeout(function(){ t = 0; try { names(); } catch (e) {} }, 1000); })
          .observe(document.documentElement, { childList: true, subtree: true });
      }
      if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start); else start();
    })();
  """

  /** Headers for a page formicaria opens itself. */
  fun headers(url: String): Map<String, String> =
    if (gpc && !off(url)) mapOf("Sec-GPC" to "1") else emptyMap()

  /** The page shown when a site has no secure version. */
  fun insecurePage(http: String): String {
    val safe = http.replace("&", "&amp;").replace("<", "&lt;").replace("\"", "&quot;")
    return """<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{font:16px system-ui,sans-serif;margin:2rem 1.2rem;line-height:1.5}a{display:inline-block;margin-top:1rem}</style>
<h2>No secure connection</h2><p><b>${Uri.parse(http).host ?: ""}</b> does not offer a secure (https) version of this page.
What you send or see there can be read on the way.</p>
<p><a href="fm-insecure:$safe">Open it anyway, this time</a></p>"""
  }
}

/** **The blocking engine** in the app's Rust library (`mobile/src-tauri/src/block.rs`): asked for
 *  every request a page makes. Answers "allow" until its lists are ready. */
object Blocker {
  @JvmStatic external fun configure(trackers: Boolean, ads: Boolean, links: Boolean)
  @JvmStatic external fun configureApps(apps: Boolean, around: Boolean)
  /** `""` let through, `"-"` refused, otherwise a stand-in: `<media type>\n<base64>`. */
  @JvmStatic external fun verdict(url: String, page: String, kind: String): String
  @JvmStatic external fun cleanLink(url: String): String
  @JvmStatic external fun hide(url: String): String
  @JvmStatic external fun hideNames(url: String, classes: String, ids: String): String
  @JvmStatic external fun scriptlets(url: String): String
  @JvmStatic external fun procedural(url: String): String
  @JvmStatic external fun hostBlocked(host: String): Boolean
  @JvmStatic external fun ready(): Boolean
  @JvmStatic external fun state(): String
  @JvmStatic external fun appsReady(): Boolean
  @JvmStatic external fun dnsName(q: ByteArray): String
  @JvmStatic external fun dnsRefuse(q: ByteArray): ByteArray
  @JvmStatic external fun dnsCloaked(answer: ByteArray): Boolean
  @JvmStatic external fun dnsTtl(answer: ByteArray): Int
  @JvmStatic external fun dnsWithId(answer: ByteArray, question: ByteArray): ByteArray

  /** A request's answer from its verdict: null to let it through. */
  fun response(v: String): android.webkit.WebResourceResponse? {
    if (v.isEmpty()) return null
    if (v == "-") return android.webkit.WebResourceResponse("text/plain", "utf-8", java.io.ByteArrayInputStream(ByteArray(0)))
    val mime = v.substringBefore('\n')
    val bytes = try { android.util.Base64.decode(v.substringAfter('\n'), android.util.Base64.DEFAULT) } catch (e: Exception) { ByteArray(0) }
    val type = mime.substringBefore(';').trim()
    return android.webkit.WebResourceResponse(type, if (type.startsWith("text/") || type.contains("javascript")) "utf-8" else null, java.io.ByteArrayInputStream(bytes))
  }

  /** A request's type as the lists name them: the page view's own word (`Sec-Fetch-Dest`) when it
   *  gives one, otherwise read from what is asked for. */
  fun kind(url: String, accept: String?, mainFrame: Boolean, dest: String? = null): String {
    if (mainFrame) return "document"
    when (dest) {
      "script", "worker", "sharedworker", "serviceworker" -> return "script"
      "image" -> return "image"
      "style" -> return "stylesheet"
      "font" -> return "font"
      "audio", "video", "track" -> return "media"
      "iframe", "frame" -> return "sub_frame"
      "object", "embed" -> return "object"
      "document" -> return "sub_frame"
      "empty" -> return "xmlhttprequest"
    }
    val path = url.substringBefore('?').substringBefore('#').lowercase()
    val a = accept ?: ""
    return when {
      a.startsWith("text/css") || path.endsWith(".css") -> "stylesheet"
      a.startsWith("image/") || Regex("\\.(png|jpe?g|gif|webp|avif|svg|ico)$").containsMatchIn(path) -> "image"
      path.endsWith(".js") || path.endsWith(".mjs") -> "script"
      Regex("\\.(woff2?|ttf|otf)$").containsMatchIn(path) -> "font"
      Regex("\\.(mp4|webm|m3u8|mpd|mp3|m4a|ogg|opus)$").containsMatchIn(path) -> "media"
      a.contains("text/html") -> "sub_frame"
      else -> "xmlhttprequest"
    }
  }
}

/** What this build carries: the phone-wide tracker block is in the one APK, for everyone (owner, 2026-11-17). */
object BuildFlags { const val VPN = true }

/** What the protections did on one page, for the Privacy sheet. */
class PageCounts { var blocked = 0; var cleaned = 0; var upgraded = 0; var banners = 0
  fun json(id: String): JSONObject = JSONObject().put("id", id).put("blocked", blocked)
    .put("cleaned", cleaned).put("upgraded", upgraded).put("banners", banners)
}
