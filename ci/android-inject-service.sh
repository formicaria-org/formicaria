#!/bin/sh
# Inject a **foreground service** into the generated Android project so Android keeps formicaria's
# process — and its network — alive while the study assistant downloads its model and answers
# questions. Without it, Doze/App-Standby throttle the backgrounded app's network (the ~150 MB model
# download stalls with DNS failures — seen on-device) and LMKD is free to reap the model subprocess
# mid-generation.
#
# gen/android is generated (and gitignored), so — like ci/android-stage-runtime.sh — this committed
# script re-applies the edits idempotently. `MainActivity.kt` lives outside `generated/`, so tauri does
# not overwrite it; we own it. Run before an android build (android-apk/android-release depend on it).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
pkg_dir="$root/mobile/src-tauri/gen/android/app/src/main/java/dev/formicaria/notes"
manifest="$root/mobile/src-tauri/gen/android/app/src/main/AndroidManifest.xml"
[ -d "$pkg_dir" ] || { echo "android-inject-service: $pkg_dir missing — run 'tauri android init' first" >&2; exit 1; }
[ -f "$manifest" ] || { echo "android-inject-service: $manifest missing" >&2; exit 1; }

# 1) The service itself. Overwrite each run (idempotent by construction).
cat > "$pkg_dir/AgentService.kt" <<'KT'
package dev.formicaria.notes

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder

// A foreground service so Android keeps formicaria's process (and its network) alive while the study
// assistant downloads its model and answers questions. See ci/android-inject-service.sh for why.
class AgentService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val channelId = "formicaria_agent"
        val notification: Notification =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                val chan = NotificationChannel(channelId, "Study assistant", NotificationManager.IMPORTANCE_LOW)
                getSystemService(NotificationManager::class.java).createNotificationChannel(chan)
                Notification.Builder(this, channelId)
                    .setContentTitle("formicaria study assistant")
                    .setContentText("Running on your device")
                    .setSmallIcon(android.R.drawable.stat_sys_download)
                    .setOngoing(true)
                    .build()
            } else {
                @Suppress("DEPRECATION")
                Notification.Builder(this)
                    .setContentTitle("formicaria study assistant")
                    .setContentText("Running on your device")
                    .setSmallIcon(android.R.drawable.stat_sys_download)
                    .setOngoing(true)
                    .build()
            }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            startForeground(1, notification)
        }
        // **NOT_STICKY, deliberately — it used to be STICKY and that resurrected a lie.** Android
        // restarts a sticky service after killing its process, and it restarts *only the service*:
        // no Activity, so `WryActivity.onCreate` never runs, so `Rust.create()` never runs, so there
        // is no vault, no dispatch and no model in that process. What the user got was a permanent
        // "formicaria study assistant — Running on your device" notification over an empty JVM,
        // holding the foreground-service priority that makes LMKD reap other apps first (on the
        // user's phone it was outliving Chrome and the Play Store). MainActivity starts this
        // service on every launch, so stickiness bought nothing a tap does not.
        return START_NOT_STICKY
    }

    // **Swiping formicaria away from Recents ends it — this is what makes that true.** With this
    // service running, Android does not kill the process when the task is removed: it destroys the
    // Activity and keeps the process. Tauri builds its webview once per process, for the Activity it
    // started with, so the next tap created a new Activity in the old process and nothing ever drew
    // in it — a white screen until a force-stop (`decisions.md#track-m`, 2026-10-24). On other runs
    // Android killed the process and at once restarted it for this service alone, and the next tap
    // opened into that and never answered. Stopping the service first is what keeps it from being
    // restarted; ending the process is what keeps the next tap a cold start. The model children
    // follow by `PR_SET_PDEATHSIG`. The page has already been told it is hidden (`visibilitychange`),
    // which is when it flushes a pending save.
    override fun onTaskRemoved(rootIntent: Intent?) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(Service.STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
        android.os.Process.killProcess(android.os.Process.myPid())
    }
}
KT

# 1b) No reminders on Android (`decisions.md#track-m`, 2026-10-31, *the phone sets no alarms and
#     starts at no boot*). `Reminders.kt` — wake-up alarms, a start-at-boot receiver, notifications
#     the page wrote — came with 0.7.7, when Play Protect began to call the app harmful. A copy an
#     earlier run left beside `MainActivity.kt` is removed, or Gradle would still compile it.
rm -f "$pkg_dir/Reminders.kt"

# 2) MainActivity starts the service on launch. Overwrite the whole file (small, stable, and tauri
#    does not regenerate it — it is outside generated/).
cat > "$pkg_dir/MainActivity.kt" <<'KT'
package dev.formicaria.notes

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Base64
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.webkit.CookieManager
import android.webkit.SslErrorHandler
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebStorage
import android.webkit.WebViewClient
import android.net.http.SslError
import androidx.webkit.ScriptHandler
import androidx.media3.common.Player
import android.widget.FrameLayout
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.json.JSONTokener

class MainActivity : TauriActivity() {
  private var wv: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    // Extend into the cutout on both short edges, so the inset the window reports is the real
    // one rather than a letterboxed zero. Without this the page can be told there is nothing
    // there while the camera is sitting on top of it.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      window.attributes.layoutInDisplayCutoutMode =
        WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
    }
    recordInstaller()
    super.onCreate(savedInstanceState)
    // Keep the app + its network alive for the study assistant's download and inference.
    val svc = Intent(this, AgentService::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) startForegroundService(svc) else startService(svc)
    takeShared(intent)
  }

  /// **Which app installed formicaria** (the installer of record), written to `cacheDir/installer`
  /// at every start and before `super.onCreate`, so the Rust side has it when it starts. An app store
  /// keeps formicaria up to date itself, so the phone's own GitHub check starts off for a store
  /// install (`decisions.md` 2026-10-30, *the Android update check follows who installed the app*);
  /// `update.rs::installer_file` reads the same literal path. Empty when Android names nobody.
  /// Failure is silent and harmless: no file means today's default, the check on.
  private fun recordInstaller() {
    try {
      val who = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        packageManager.getInstallSourceInfo(packageName).installingPackageName
      } else {
        @Suppress("DEPRECATION")
        val name = packageManager.getInstallerPackageName(packageName)
        name
      }
      File(cacheDir, "installer").writeText(who ?: "")
    } catch (e: Exception) {
    }
  }

  // `singleTask`: a share while the app is open arrives here rather than as a second app.
  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    takeShared(intent)
  }

  /// **What another app shared to formicaria** (`decisions.md#ui`, 2026-10-20): Android's Share
  /// button under a screenshot, or in the gallery. The page cannot read a `content://` address,
  /// and the app that shared it may take the permission back once it is no longer in front, so
  /// the bytes are copied at once into the app's own cache and held here until the page asks
  /// (`__fmShare`). At a cold start the page does not exist yet, which is why it asks rather than
  /// being told; `fm-share` only says there is more to ask for.
  private class Shared(val file: File, val name: String, val mime: String)
  private val shared = mutableListOf<Shared>()
  /// What could not be taken, and why: `too large` or `could not be read`.
  private val refused = mutableListOf<Pair<String, String>>()

  /// **Text brought in from another app** (`decisions.md` 2026-11-16, *the phone's Bring in*): what
  /// Share hands over as text (a page's link, a passage), and what a person selected in any app and
  /// chose *formicaria* for in the selection menu (`PROCESS_TEXT`). Held for the page like files.
  private class SharedText(val text: String, val subject: String)
  private val texts = mutableListOf<SharedText>()

  private fun takeShared(intent: Intent?) {
    val said: CharSequence? = when (intent?.action) {
      Intent.ACTION_PROCESS_TEXT -> intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)
      Intent.ACTION_SEND ->
        if (intent.type?.startsWith("text/") == true && streamOf(intent) == null) intent.getCharSequenceExtra(Intent.EXTRA_TEXT)
        else null
      else -> null
    }
    if (said != null) {
      setIntent(Intent(this, MainActivity::class.java))
      val subject = intent?.getStringExtra(Intent.EXTRA_SUBJECT) ?: ""
      val text = said.toString().take(MAX_TEXT)
      if (text.isNotBlank()) synchronized(shared) { texts.add(SharedText(text, subject)) }
      wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null)
      return
    }
    val uris: List<Uri> = when (intent?.action) {
      Intent.ACTION_SEND -> listOfNotNull(streamOf(intent))
      Intent.ACTION_SEND_MULTIPLE -> streamsOf(intent)
      else -> return
    }
    // Taken once: a rotation recreates the activity with the intent it was started with.
    setIntent(Intent(this, MainActivity::class.java))
    if (uris.isEmpty()) return
    Thread {
      val dir = File(cacheDir, "shared").apply { mkdirs() }
      for (uri in uris) {
        val (name, size) = described(uri)
        try {
          // Read through the page as text, so a bound is kept: a phone video would not fit.
          if (size > MAX_SHARE) { synchronized(shared) { refused.add(Pair(name, "too large")) }; continue }
          val out = File(dir, "${System.nanoTime()}-${name.replace(Regex("[^A-Za-z0-9._-]"), "_")}")
          contentResolver.openInputStream(uri)?.use { input -> out.outputStream().use { input.copyTo(it) } }
          if (!out.isFile || out.length() > MAX_SHARE) {
            val why = if (out.isFile) "too large" else "could not be read"
            out.delete()
            synchronized(shared) { refused.add(Pair(name, why)) }
            continue
          }
          val mime = contentResolver.getType(uri) ?: "application/octet-stream"
          synchronized(shared) { shared.add(Shared(out, name, mime)) }
        } catch (e: Exception) {
          // Most often the app that shared it gave no permission to read it.
          synchronized(shared) { refused.add(Pair(name, "could not be read")) }
        }
      }
      runOnUiThread { wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null) }
    }.start()
  }

  @Suppress("DEPRECATION")
  private fun streamOf(intent: Intent): Uri? =
    if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
    else intent.getParcelableExtra(Intent.EXTRA_STREAM)

  @Suppress("DEPRECATION")
  private fun streamsOf(intent: Intent): List<Uri> =
    (if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
    else intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)) ?: emptyList()

  /// The file's own name and size, as the app that shared it gives them; -1 when it does not say.
  private fun described(uri: Uri): Pair<String, Long> {
    var name = uri.lastPathSegment ?: "shared"
    var size = -1L
    try {
      contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
        ?.use { c ->
          if (c.moveToFirst()) {
            val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val s = c.getColumnIndex(OpenableColumns.SIZE)
            if (n >= 0 && !c.isNull(n)) name = c.getString(n)
            if (s >= 0 && !c.isNull(s)) size = c.getLong(s)
          }
        }
    } catch (e: Exception) {
      // Some apps share without answering the query; the copy still goes ahead.
    }
    return Pair(name, size)
  }

  /// The page's side of a share: what is held, each file's bytes, and that it has them. `done`
  /// takes counts, not "all": a second share still being copied while the page reads the first
  /// must not be thrown away with it.
  inner class Share {
    /// **Open with…** (2026-11-17): a song's or a video's file handed to another app to play — the
    /// vault's own file, read through a content link, never copied. Only files inside the app's
    /// storage are offered (`file_paths.xml`'s `files-path`).
    @JavascriptInterface
    fun openWith(path: String, mime: String) {
      runOnUiThread {
        try {
          val f = File(path)
          if (!f.canonicalPath.startsWith(filesDir.canonicalPath)) return@runOnUiThread
          val uri = androidx.core.content.FileProvider.getUriForFile(this@MainActivity, "$packageName.fileprovider", f)
          val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime.ifEmpty { "*/*" })
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
          startActivity(Intent.createChooser(view, "Open with"))
        } catch (e: Exception) { }
      }
    }

    @JavascriptInterface
    fun pending(): String = synchronized(shared) {
      val files = JSONArray()
      for (f in shared) files.put(JSONObject().put("name", f.name).put("mime", f.mime))
      val no = JSONArray()
      for ((name, why) in refused) no.put(JSONObject().put("name", name).put("why", why))
      val said = JSONArray()
      for (t in texts) said.put(JSONObject().put("text", t.text).put("subject", t.subject))
      val got = JSONArray()
      synchronized(brought) { for (b in brought) got.put(b) }
      JSONObject().put("files", files).put("refused", no).put("texts", said).put("brought", got).toString()
    }

    /// **A note handed to another app** (`workspace-design` §13.6): Android's share sheet with the note's
    /// text; the person picks where it goes.
    @JavascriptInterface
    fun out(text: String, subject: String) {
      val send = Intent(Intent.ACTION_SEND).setType("text/plain")
        .putExtra(Intent.EXTRA_TEXT, text.take(MAX_TEXT)).putExtra(Intent.EXTRA_SUBJECT, subject)
      runOnUiThread { try { startActivity(Intent.createChooser(send, null)) } catch (e: Exception) { } }
    }

    /// The page has the pieces brought from a page screen (by count).
    @JavascriptInterface
    fun doneBrought(n: Int) {
      synchronized(brought) { repeat(minOf(n, brought.size)) { brought.removeAt(0) } }
    }

    /// The page has the texts it was given (by count, as for files).
    @JavascriptInterface
    fun doneTexts(n: Int) {
      synchronized(shared) { repeat(minOf(n, texts.size)) { texts.removeAt(0) } }
    }

    @JavascriptInterface
    fun read(i: Int): String {
      val f = synchronized(shared) { shared.getOrNull(i) } ?: return ""
      return Base64.encodeToString(f.file.readBytes(), Base64.NO_WRAP)
    }

    @JavascriptInterface
    fun done(files: Int, refusedCount: Int) {
      synchronized(shared) {
        repeat(minOf(files, shared.size)) { shared.removeAt(0).file.delete() }
        repeat(minOf(refusedCount, refused.size)) { refused.removeAt(0) }
      }
    }
  }

  /// The insets the window last reported, in CSS pixels. Held so the page can **ask** for them.
  // `Float`, because `displayMetrics.density` is one and the division follows it.
  private var top = 0f
  private var right = 0f
  private var bottom = 0f
  private var left = 0f
  /// The on-screen keyboard **beyond the navigation bar**, in CSS pixels; 0 while it is closed.
  ///
  /// Edge-to-edge at SDK 35+ means Android no longer resizes the window for the keyboard — it is drawn
  /// over the WebView, and the page believes the whole screen is visible, so the line being typed sat
  /// under it (the owner, 2026-10-08). The page subtracts this from its height (`--app-h`). Beyond the
  /// navigation bar because the page already pays that as `--safe-bottom`.
  private var keyboard = 0f

  /// **The page asks; the shell does not tell.**
  ///
  /// `env(safe-area-inset-*)` is 0 in wry's WebView — it never forwards the window insets — so the
  /// stylesheet was guessing: 1.75rem against a camera cutout that is really 52px, and 0.5rem
  /// against a navigation bar that is really 47px.
  ///
  /// The first attempt at this pushed the values in with `evaluateJavascript` from the inset
  /// listener, and it never landed. `onWebViewCreate` runs *before* wry issues the first
  /// `loadUrl` and before `setContentView`, so the first inset dispatch — and the `post` after it
  /// — happen against `about:blank`, and the inline properties die when the real document commits.
  /// It looked intermittent because it was a race.
  ///
  /// So this copies the shape `FM_CONFIG_DIR` already uses: **put the platform fact where the
  /// consumer already looks, and let the consumer read it when it is ready.** A registered
  /// JavaScript interface is re-injected into every document Android loads, so it is present for
  /// the first page and survives every reload — and wry uses exactly this mechanism for its own
  /// IPC bridge, so it is native to this stack rather than a new idea.
  /// **The device's own status-bar height, in CSS pixels** — the fallback, and not a guess.
  ///
  /// **A zero here defeated the stylesheet's fallback rather than falling back to it**, which is
  /// the whole bug and is worth stating exactly. `top` is 0 until the first inset dispatch, and
  /// `addDocumentStartJavaScript` runs `APPLY_INSETS` at *document start* — so the page opened with
  /// `--safe-top: 0px` written as an **inline style on `:root`**, and an inline style outranks the
  /// `@media (pointer: coarse)` floor completely. The floor was therefore never in force on Android
  /// at all: not 28px, but nothing. Confirmed against the shipped layout — the top control rendered
  /// at `y = 0`, its full height inside a camera cutout that measures `DisplayCutout
  /// insets=Rect(0, 130 - 0, 0)` / density 3.25 = **40 CSS px**. Reported 2026-09-08:
  /// *"you are continuing to use the top part of the screen which is untouchable and passes through
  /// the frontal camera… we cannot use top pixels."*
  ///
  /// The general shape, worth more than the instance: **a bridge that publishes a placeholder
  /// overrides the fallback it was meant to complement.** Either report nothing until you know, or
  /// report something true — never a zero that outranks a guess.
  ///
  /// Android has always known the real number, so ask it rather than picking one: `status_bar_height`
  /// is a platform dimen resource, available immediately and correct per device. A phone whose
  /// status bar is shorter than its cutout would still be wrong, so the cutout is unioned in where
  /// the API exists — this returns the larger of the two, which is what `systemBars() or
  /// displayCutout()` will report once it does dispatch.
  private fun fallbackTop(): Float {
    val d = resources.displayMetrics.density
    val id = resources.getIdentifier("status_bar_height", "dimen", "android")
    val bar = if (id > 0) resources.getDimensionPixelSize(id) / d else 0f
    val cut =
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        (window?.decorView?.rootWindowInsets?.displayCutout?.safeInsetTop ?: 0) / d
      } else {
        0f
      }
    return maxOf(bar, cut)
  }

  inner class Insets {
    @JavascriptInterface
    fun json(): String {
      // **Never report a zero top before the first dispatch.** Zero is indistinguishable from
      // "this device has no status bar", and the page cannot tell the difference either — so the
      // one reading that must never be guessed downwards is guessed from the device itself.
      val t = if (top > 0f) top else fallbackTop()
      return "{\"top\":$t,\"right\":$right,\"bottom\":$bottom,\"left\":$left,\"keyboard\":$keyboard}"
    }
  }

  /// **Updating hands the newest APK to the browser, and Android's installer takes it from there**
  /// (`decisions.md#track-m`, 2026-10-31, *a phone update opens in the browser*).
  ///
  /// The app used to download, verify and install its own update, which needs
  /// `REQUEST_INSTALL_PACKAGES` — a permission scanners read as a red flag, and the next
  /// suspect once the restart receiver alone did not clear *"Harmful app blocked"*. Now this opens
  /// one fixed address, the newest release's APK, in the browser; the person taps **Open** when it
  /// arrives, exactly as on the first install (`docs/src/user/android.md`). Android still refuses an
  /// update signed by anyone else.
  ///
  /// **It takes no argument**, as before: nothing running in the page can name another address.
  /// Returns `ok` once the browser has been asked. A phone with no browser at all is not reported:
  /// the failure happens on the UI thread, after this has answered.
  inner class UpdateDownload {
    @JavascriptInterface
    fun download(): String {
      val view = Intent(Intent.ACTION_VIEW, Uri.parse(LATEST_APK)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      runOnUiThread { try { startActivity(view) } catch (e: Exception) { } }
      return "ok"
    }
  }

  /// **A web page, an item of the collection** (`docs/context/workspace-design-2026-11-15.md` §13.7):
  /// the app owns the collection, the switch and the layout; the shell only **draws** a page's
  /// WebView over the box the app gives (CSS pixels of the app's own view), hides it when the app
  /// says it is out of sight, and tells the app the page's title and address as they change. One
  /// WebView per item, kept while the item is open, so a page switched away from and back is where
  /// it was. A page is the web's: no bridge into it, nothing of formicaria's inside it.
  private val pages = HashMap<String, WebView>()
  /** The page item drawn last (the one in front when two are shown together). */
  private var lastShown: String? = null

  /** What the protections did on each page (the Privacy sheet), and the scripts each page got. */
  private val counts = HashMap<String, PageCounts>()
  private val scripts = HashMap<String, List<ScriptHandler>>()
  /** The media files and streams each page fetched (Save media), the last 200. */
  private val seenMedia = java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.ConcurrentLinkedDeque<String>>()
  private val MEDIA_URL = Regex("\\.(m3u8|mpd|mp4|m4v|webm|mov|mp3|m4a|aac|ogg|oga|opus|flac|wav)(\\?|$)", RegexOption.IGNORE_CASE)
  /** An `https` address formicaria tried in place of an `http` one → the `http` one. */
  private val upgradedFrom = java.util.concurrent.ConcurrentHashMap<String, String>()

  /** Each page's address, readable from the threads its requests are asked on. */
  private val pageUrls = java.util.concurrent.ConcurrentHashMap<String, String>()
  private val countsDue = HashSet<String>()
  /** The count told at most twice a second, however many requests are refused. */
  private fun tellCountsSoon(id: String) {
    if (!countsDue.add(id)) return
    wv?.postDelayed({ countsDue.remove(id); tellCounts(id) }, 500)
  }

  private fun tellCounts(id: String) {
    val d = (counts[id] ?: return).json(id)
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-privacy',{detail:$d}))", null)
  }

  /** The address a page should really open: tracking parameters off, made secure. Null if as is. */
  private fun rewrite(id: String, url: String): String? {
    val c = counts.getOrPut(id) { PageCounts() }
    var next = url
    PagePrivacy.clean(next)?.let { next = it; c.cleaned++ }
    PagePrivacy.upgrade(next)?.let { upgradedFrom[it] = next; next = it; c.upgraded++ }
    if (next == url) return null
    tellCounts(id)
    return next
  }

  private fun open(id: String, page: WebView, url: String) {
    val next = rewrite(id, url) ?: url
    page.loadUrl(next, PagePrivacy.headers(next))
  }

  /** **A saved page** (a note's copy of one): read from the vault's own file, shown as it was saved,
   *  at its own address, so its links lead where they led. Only files in the app's storage. */
  private fun openSaved(page: WebView, path: String) {
    try {
      val f = File(path)
      if (!f.canonicalPath.startsWith(filesDir.canonicalPath) || f.length() > 60L * 1024 * 1024) return
      val html = f.readText()
      val base = Regex("<base[^>]+href=[\"']([^\"']+)", RegexOption.IGNORE_CASE).find(html)?.groupValues?.get(1)
      page.loadDataWithBaseURL(base, html, "text/html", "utf-8", null)
    } catch (e: Exception) { }
  }

  /** The scripts a page gets at its start, as the switches stand now (re-given when they change). */
  private fun giveScripts(id: String, page: WebView) {
    scripts.remove(id)?.forEach { it.remove() }
    if (!WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) return
    scripts[id] = PagePrivacy.startScripts().map { WebViewCompat.addDocumentStartJavaScript(page, it, setOf("*")) }
  }

  /** The fire button: every page's cookies, stored data, cache and history, not formicaria's own. */
  private fun forget() {
    CookieManager.getInstance().removeAllCookies(null)
    CookieManager.getInstance().flush()
    val mine = wv?.url?.let { Uri.parse(it) }?.let { "${it.scheme}://${it.authority}" }
    WebStorage.getInstance().getOrigins { m ->
      for (o in (m as? Map<*, *>)?.keys ?: emptySet<Any>()) {
        val s = o.toString()
        if (mine == null || !s.startsWith(mine)) WebStorage.getInstance().deleteOrigin(s)
      }
    }
    pages.values.forEach { it.clearCache(true); it.clearHistory(); it.clearFormData() }
    android.webkit.WebViewDatabase.getInstance(this).clearHttpAuthUsernamePassword()
    counts.clear()
  }

  /** A page's own counter: the cookie pop-up script tells it each banner refused. Nothing else. */
  inner class PageCounter(private val id: String) {
    @JavascriptInterface
    fun banner() {
      runOnUiThread { counts.getOrPut(id) { PageCounts() }.banners++; tellCounts(id) }
    }

    /// The lists' hide rules for the page (its own) and for the names it uses (the generic ones).
    @JavascriptInterface
    fun hide(url: String): String = if (PagePrivacy.off(url)) "" else try { Blocker.hide(url) } catch (e: Throwable) { "" }

    @JavascriptInterface
    fun hideNames(url: String, classes: String, ids: String): String =
      if (PagePrivacy.off(url)) "" else try { Blocker.hideNames(url, classes, ids) } catch (e: Throwable) { "" }
  }

  private fun tellApp(id: String, page: WebView) {
    val d = JSONObject().put("id", id).put("title", page.title ?: "").put("url", page.url ?: "")
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page',{detail:$d}))", null)
  }

  private fun pageFor(id: String, url: String): WebView {
    pages[id]?.let { return it }
    val page = WebView(this)
    page.settings.javaScriptEnabled = true
    page.settings.domStorageEnabled = true
    page.settings.builtInZoomControls = true
    page.settings.displayZoomControls = false
    page.webViewClient = object : WebViewClient() {
      override fun doUpdateVisitedHistory(view: WebView, u: String?, reload: Boolean) {
        pageUrls[id] = view.url ?: ""
        tellApp(id, view)
      }

      override fun onPageStarted(view: WebView, u: String?, icon: android.graphics.Bitmap?) {
        pageUrls[id] = u ?: ""
        counts[id]?.let { it.blocked = 0; it.banners = 0 }
      }

      /// A link followed: tracking parameters off and made secure (the Privacy switches), and the
      /// one way back to a site without a secure version, chosen on its page.
      override fun shouldOverrideUrlLoading(view: WebView, req: WebResourceRequest): Boolean {
        val u = req.url.toString()
        if (u.startsWith("fm-insecure:")) {
          val http = u.removePrefix("fm-insecure:")
          PagePrivacy.insecureOk += PagePrivacy.host(http)
          view.loadUrl(http, PagePrivacy.headers(http))
          return true
        }
        if (!req.isForMainFrame || req.method != "GET") return false
        val next = rewrite(id, u) ?: return false
        view.loadUrl(next, PagePrivacy.headers(next))
        return true
      }

      /// Every request the page makes: media ones are remembered for Save media.
      override fun shouldInterceptRequest(view: WebView, req: WebResourceRequest): android.webkit.WebResourceResponse? {
        val u = req.url.toString()
        val pageUrl = pageUrls[id] ?: ""
        if ((PagePrivacy.trackers || PagePrivacy.ads) && !PagePrivacy.off(pageUrl) && !req.isForMainFrame) {
          val kind = Blocker.kind(u, req.requestHeaders["Accept"], false)
          if (try { Blocker.check(u, pageUrl, kind) } catch (e: Throwable) { false }) {
            runOnUiThread { counts.getOrPut(id) { PageCounts() }.blocked++; tellCountsSoon(id) }
            return android.webkit.WebResourceResponse("text/plain", "utf-8", java.io.ByteArrayInputStream(ByteArray(0)))
          }
        }
        if (MEDIA_URL.containsMatchIn(u)) {
          val q = seenMedia.getOrPut(id) { java.util.concurrent.ConcurrentLinkedDeque() }
          q.add(u)
          while (q.size > 200) q.pollFirst()
        }
        return null
      }

      override fun onReceivedError(view: WebView, req: WebResourceRequest, err: WebResourceError) {
        if (!req.isForMainFrame) return
        val http = upgradedFrom.remove(req.url.toString()) ?: return
        view.loadDataWithBaseURL(null, PagePrivacy.insecurePage(http), "text/html", "utf-8", null)
      }

      override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
        handler.cancel()
        val http = upgradedFrom.remove(error.url ?: "") ?: return
        view.loadDataWithBaseURL(null, PagePrivacy.insecurePage(http), "text/html", "utf-8", null)
      }
    }
    page.addJavascriptInterface(PageCounter(id), "__fmPagePrivacy")
    giveScripts(id, page)
    page.webChromeClient = object : WebChromeClient() {
      override fun onReceivedTitle(view: WebView, title: String?) = tellApp(id, view)
    }
    // **A long press on an image or a link offers it** (text keeps Android's own selection): bring it
    // into the note, or open the link as a page item of its own.
    page.setOnLongClickListener {
      val hit = page.hitTestResult
      val src = hit.extra
      if (src == null || !(src.startsWith("https://") || src.startsWith("http://"))) return@setOnLongClickListener false
      when (hit.type) {
        WebView.HitTestResult.IMAGE_TYPE, WebView.HitTestResult.SRC_IMAGE_ANCHOR_TYPE -> { offer(page, "image", src); true }
        WebView.HitTestResult.SRC_ANCHOR_TYPE -> { offer(page, "link", src); true }
        else -> false
      }
    }
    page.visibility = View.GONE
    findViewById<ViewGroup>(android.R.id.content)
      .addView(page, FrameLayout.LayoutParams(0, 0))
    pages[id] = page
    if (url.startsWith("https://") || url.startsWith("http://")) open(id, page, url)
    else if (url.startsWith("fmsaved:")) openSaved(page, url.removePrefix("fmsaved:"))
    return page
  }

  private fun offer(page: WebView, kind: String, src: String) {
    val items = if (kind == "link") arrayOf("Bring the link into your note", "Open it as a page")
      else arrayOf("Bring the image into your note")
    android.app.AlertDialog.Builder(this).setItems(items) { _, which ->
      if (which == 0) {
        bring(JSONObject().put("kind", kind).put("src", src).put("pageUrl", page.url ?: "")
          .put("pageTitle", page.title ?: "").put("note", ""))
      } else {
        val d = JSONObject().put("url", src)
        wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-open',{detail:$d}))", null)
      }
    }.show()
  }

  /// **Bring into note, first in Android's selection bar** (`decisions.md` 2026-11-17, *one frame*;
  /// the owner: *"on the phone I have it not as the main thing, only after scrolling the options"*).
  /// For a selection in formicaria's own screens or in a page it draws, the item goes in ahead of the
  /// system's; from other apps Android orders outside items late, which formicaria cannot change.
  override fun onActionModeStarted(mode: android.view.ActionMode) {
    super.onActionModeStarted(mode)
    val menu = mode.menu
    if (menu.findItem(BRING_ITEM) != null) return
    val page = pages.entries.firstOrNull { it.value.visibility == View.VISIBLE && it.value.hasFocus() }?.key
    menu.add(Menu.NONE, BRING_ITEM, 0, "Bring into note")
      .setShowAsActionFlags(MenuItem.SHOW_AS_ACTION_ALWAYS)
      .setOnMenuItemClickListener {
        if (page != null) Pages().bring(page, "")
        else wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-bring-selection'))", null)
        mode.finish()
        true
      }
  }

  /// **Trackers in other apps** (`TrackerVpn`): Android asks the person once; then the switch starts
  /// and stops it. The apps let through are kept here, and the block restarts without them.
  private val vpnAsk = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.StartActivityForResult()) { r ->
    if (r.resultCode == RESULT_OK) startService(Intent(this, TrackerVpn::class.java))
    else wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-vpn',{detail:{refused:true}}))", null)
  }
  private fun vpnWanted(on: Boolean) {
    TrackerVpn.allowed = getSharedPreferences("vpn", MODE_PRIVATE).getStringSet("allowed", emptySet())!!.toSet()
    if (on == TrackerVpn.on) return
    if (!on) { startService(Intent(this, TrackerVpn::class.java).setAction(TrackerVpn.STOP)); return }
    val ask = android.net.VpnService.prepare(this)
    if (ask != null) vpnAsk.launch(ask) else startService(Intent(this, TrackerVpn::class.java))
  }
  inner class VpnShell {
    @JavascriptInterface
    fun state(): String = TrackerVpn.state { pkg ->
      try { packageManager.getApplicationLabel(packageManager.getApplicationInfo(pkg, 0)).toString() } catch (e: Exception) { pkg }
    }
    /// Let an app through (its trackers no longer refused), or refuse them again.
    @JavascriptInterface
    fun allow(pkg: String, through: Boolean) {
      val prefs = getSharedPreferences("vpn", MODE_PRIVATE)
      val set = prefs.getStringSet("allowed", emptySet())!!.toMutableSet()
      if (through) set.add(pkg) else set.remove(pkg)
      prefs.edit().putStringSet("allowed", set).apply()
      runOnUiThread {
        if (TrackerVpn.on) {
          startService(Intent(this@MainActivity, TrackerVpn::class.java).setAction(TrackerVpn.STOP))
          wv?.postDelayed({ vpnWanted(true) }, 600)
        } else TrackerVpn.allowed = set
      }
    }
  }

  /// **The native player** (`PlaybackService`): the app hands it the queue and presses its keys; it
  /// says what it is doing (`fm-player`), twice a second while playing.
  inner class PlayerShell {
    private fun withPlayer(f: (androidx.media3.exoplayer.ExoPlayer) -> Unit) {
      runOnUiThread {
        val p = PlaybackService.player
        if (p != null) f(p)
        else {
          PlaybackService.pending = { q -> listen(q); f(q) }
          startService(Intent(this@MainActivity, PlaybackService::class.java))
        }
      }
    }

    /// The queue (`[{path, title, artist, album, cover}]`), starting at `index`, at `seconds`.
    @JavascriptInterface
    fun setQueue(json: String, index: Int, seconds: Double, play: Boolean) = withPlayer { p ->
      val a = JSONArray(json)
      val items = (0 until a.length()).map { i ->
        val o = a.getJSONObject(i)
        PlaybackService.item(o.getString("path"), o.optString("title"), o.optString("artist"), o.optString("album"),
          o.optString("cover").takeIf { it.isNotEmpty() })
      }
      p.setMediaItems(items, index.coerceIn(0, (items.size - 1).coerceAtLeast(0)), (seconds * 1000).toLong())
      p.prepare()
      p.playWhenReady = play
      listen(p)
    }
    @JavascriptInterface fun play() = withPlayer { it.play() }
    @JavascriptInterface fun pause() = withPlayer { it.pause() }
    @JavascriptInterface fun next() = withPlayer { if (it.hasNextMediaItem()) it.seekToNextMediaItem() }
    @JavascriptInterface fun previous() = withPlayer { it.seekToPrevious() }
    @JavascriptInterface fun jump(index: Int) = withPlayer { it.seekTo(index, 0); it.play() }
    @JavascriptInterface fun seek(seconds: Double) = withPlayer { it.seekTo((seconds * 1000).toLong()) }
    @JavascriptInterface fun volume(v: Double) = withPlayer { it.volume = v.toFloat().coerceIn(0f, 1f) }
    @JavascriptInterface fun modes(shuffle: Boolean, repeat: String) = withPlayer {
      it.shuffleModeEnabled = shuffle
      it.repeatMode = when (repeat) { "all" -> Player.REPEAT_MODE_ALL; "one" -> Player.REPEAT_MODE_ONE; else -> Player.REPEAT_MODE_OFF }
    }
    @JavascriptInterface fun stop() = withPlayer { it.stop(); it.clearMediaItems() }
  }

  private var listening: androidx.media3.exoplayer.ExoPlayer? = null
  private val tick = object : Runnable {
    override fun run() {
      val p = PlaybackService.player ?: return
      tellPlayer(p)
      if (p.isPlaying) wv?.postDelayed(this, 500)
    }
  }
  private fun tellPlayer(p: androidx.media3.exoplayer.ExoPlayer) {
    val d = JSONObject().put("index", p.currentMediaItemIndex).put("playing", p.isPlaying)
      .put("time", p.currentPosition / 1000.0).put("length", if (p.duration > 0) p.duration / 1000.0 else 0.0)
      .put("ended", p.playbackState == Player.STATE_ENDED).put("error", p.playerError?.message ?: "")
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-player',{detail:$d}))", null)
  }
  private fun listen(p: androidx.media3.exoplayer.ExoPlayer) {
    if (listening === p) return
    listening = p
    p.addListener(object : Player.Listener {
      override fun onEvents(player: Player, events: Player.Events) {
        tellPlayer(p)
        wv?.removeCallbacks(tick)
        if (p.isPlaying) wv?.postDelayed(tick, 500)
      }
    })
  }

  /// **Save media** (`MediaRun`): the app asks, polls, and files the result through the core.
  inner class MediaShell {
    @JavascriptInterface fun workDir(): String = MediaRun.workDir(applicationContext).absolutePath
    @JavascriptInterface fun probe(id: String, url: String) = MediaRun.probe(applicationContext, id, url)
    @JavascriptInterface fun start(id: String, url: String, args: String) = MediaRun.start(applicationContext, id, url, args)
    @JavascriptInterface fun jobs(): String = MediaRun.jobs()
    @JavascriptInterface fun cancel(id: String) = MediaRun.cancel(id)
    @JavascriptInterface fun forget(id: String) = MediaRun.forget(id)
    @JavascriptInterface fun freshen() { Thread { MediaRun.freshen(applicationContext, true) }.start() }
  }

  inner class Pages {
    /// Draw the item's page over that box (the app's CSS pixels), opening it at `url` the first time.
    @JavascriptInterface
    fun show(id: String, url: String, x: Double, y: Double, w: Double, h: Double) {
      runOnUiThread {
        val page = pageFor(id, url)
        val d = resources.displayMetrics.density
        val main = wv
        val origin = IntArray(2).also { main?.getLocationInWindow(it) }
        val root = IntArray(2).also { (page.parent as? View)?.getLocationInWindow(it) }
        val lp = FrameLayout.LayoutParams((w * d).toInt(), (h * d).toInt())
        lp.leftMargin = (x * d).toInt() + origin[0] - root[0]
        lp.topMargin = (y * d).toInt() + origin[1] - root[1]
        page.layoutParams = lp
        page.visibility = View.VISIBLE
        page.bringToFront()
        lastShown = id
      }
    }

    @JavascriptInterface
    fun hide(id: String) {
      runOnUiThread { pages[id]?.visibility = View.GONE }
    }

    /// The item was closed: its page goes for good.
    @JavascriptInterface
    fun close(id: String) {
      runOnUiThread {
        val page = pages.remove(id) ?: return@runOnUiThread
        scripts.remove(id)
        counts.remove(id)
        seenMedia.remove(id)
        pageUrls.remove(id)
        (page.parent as? ViewGroup)?.removeView(page)
        page.destroy()
      }
    }

    /// **Save this page** (≡): its HTML handed to the app, which reads it as the computer's *Save page*
    /// does and keeps it as a note (`lib/web/read.ts`).
    @JavascriptInterface
    fun html(id: String) {
      runOnUiThread {
        val page = pages[id] ?: return@runOnUiThread
        page.evaluateJavascript("document.documentElement.outerHTML") { got ->
          try {
            val html = (JSONTokener(got).nextValue() as? String ?: return@evaluateJavascript).take(MAX_PAGE)
            val d = JSONObject().put("id", id).put("url", page.url ?: "").put("title", page.title ?: "")
              .put("html", html).put("seen", JSONArray(seenMedia[id]?.toList() ?: emptyList<String>()))
            wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-html',{detail:$d}))", null)
          } catch (e: Exception) { }
        }
      }
    }

    /// **This device's privacy switches** (`privacy.svelte.ts`): taken, given to every open page, and
    /// the fire button acted on when it was pressed.
    @JavascriptInterface
    fun privacy(json: String) {
      val fire = try { PagePrivacy.update(json) } catch (e: Exception) { false }
      // The phone-wide block asks the same engine, with the trackers' list.
      try { Blocker.configure(PagePrivacy.trackers || PagePrivacy.appTrackers, PagePrivacy.ads) } catch (e: Throwable) { }
      runOnUiThread { try { vpnWanted(PagePrivacy.appTrackers) } catch (e: Throwable) { } }
      runOnUiThread {
        pages.forEach { (id, p) -> giveScripts(id, p) }
        if (fire) forget()
      }
    }

    /// A script the pages run at their start, handed over by the app (`dist/page/<name>.js`).
    @JavascriptInterface
    fun pageScript(name: String, text: String) {
      if (name == "consent") PagePrivacy.consentScript = text
      runOnUiThread { pages.forEach { (id, p) -> giveScripts(id, p) } }
    }

    /// This build can block trackers in other apps (the F-Droid build's VPN).
    @JavascriptInterface
    fun vpn(): Boolean = BuildFlags.VPN

    /// The address row: a new address (or a search) in this page, made private as any other.
    @JavascriptInterface
    fun go(id: String, url: String) {
      runOnUiThread { pages[id]?.let { open(id, it, url) } }
    }

    @JavascriptInterface
    fun reload(id: String) {
      runOnUiThread { pages[id]?.reload() }
    }

    @JavascriptInterface
    fun back(id: String) {
      runOnUiThread { pages[id]?.let { if (it.canGoBack()) it.goBack() } }
    }

    /// **Bring the selection in** (`decisions.md` 2026-11-16, *a page is a layer*): the page's
    /// selection (its HTML and words), or the page itself when nothing is selected, with the page's
    /// exact address and title, into `note` (or the note in front, decided by the app).
    @JavascriptInterface
    fun bring(id: String, note: String) {
      runOnUiThread {
        val page = pages[id] ?: return@runOnUiThread
        page.evaluateJavascript(PICK) { got ->
          try {
            val json = JSONTokener(got).nextValue() as? String ?: return@evaluateJavascript
            val piece = JSONObject(json)
            if (piece.optString("text").length > MAX_TEXT) piece.put("text", piece.optString("text").take(MAX_TEXT))
            if (piece.optString("html").length > MAX_TEXT) piece.remove("html")
            piece.put("note", note)
            bring(piece)
          } catch (e: Exception) { }
        }
      }
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    wv = webView
    webView.addJavascriptInterface(Insets(), "__fmInsets")
    // The update bridge, re-injected into every document like the insets one. See UpdateDownload.
    webView.addJavascriptInterface(UpdateDownload(), "__fmUpdate")
    // What another app shared: the page asks for it. See `takeShared`.
    webView.addJavascriptInterface(Share(), "__fmShare")
    // Web pages as items of the collection, drawn where the app says. See `Pages`.
    webView.addJavascriptInterface(Pages(), "__fmPage")
    // Trackers in other apps: their counts and the apps let through. See `VpnShell`.
    webView.addJavascriptInterface(VpnShell(), "__fmVpn")
    // The native player, playing on in the background. See `PlayerShell`.
    webView.addJavascriptInterface(PlayerShell(), "__fmPlayer")
    // Save media's runner (yt-dlp inside the app). See `MediaShell`.
    webView.addJavascriptInterface(MediaShell(), "__fmMedia")
    // **Back goes back in the page first**, as in any browser: while a page item is shown and has
    // somewhere to go back to. Otherwise Back is the app's (wry's callback, registered before this
    // one, so this one is asked first).
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        val shown = pages.filter { it.value.visibility == View.VISIBLE }
        val page = shown[lastShown] ?: shown.values.firstOrNull()
        if (page != null && page.canGoBack()) {
          page.goBack()
          return
        }
        // A page with nowhere further back: the app leaves it (back to the tab it came from), as a
        // browser does, rather than Android leaving formicaria.
        val id = pages.entries.firstOrNull { it.value === page }?.key
        if (id != null) {
          val d = JSONObject().put("id", id)
          wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-back',{detail:$d}))", null)
          return
        }
        isEnabled = false
        onBackPressedDispatcher.onBackPressed()
        isEnabled = true
      }
    })
    current = java.lang.ref.WeakReference(this)

    // **Applied before the page's own CSS**, on every navigation, so there is no frame in which
    // the app is laid out against a zero inset. Feature-gated exactly as `RustWebView.kt` gates
    // its own init scripts; without it the interface alone would still leave a flash.
    if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
      WebViewCompat.addDocumentStartJavaScript(webView, APPLY_INSETS, setOf("*"))
    }

    // Ask immediately rather than waiting for the system's own first dispatch: the sooner the real
    // numbers replace `fallbackTop()`, the smaller the window in which the page is laid out against
    // an approximation — even a good one.
    ViewCompat.requestApplyInsets(webView)

    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, insets ->
      val i = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val d = v.resources.displayMetrics.density
      top = i.top / d; right = i.right / d; bottom = i.bottom / d; left = i.left / d
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
      keyboard = maxOf(0, ime.bottom - i.bottom) / d
      // Now only the *update* channel — a rotation, the keyboard, a returned-to app. By the time
      // any of those happen the page exists, which is the one job this call can actually do.
      v.post { (v as WebView).evaluateJavascript(APPLY_INSETS, null) }
      insets
    }
  }

  // A rotation or coming back to the app can change the insets; asking again is cheap. At launch
  // `wv` is still null here — the webview arrives on wry's main-pipe message, after onResume has
  // returned — which is precisely why this cannot be the mechanism that gets the first values in.
  override fun onResume() {
    super.onResume()
    wv?.let { ViewCompat.requestApplyInsets(it) }
  }

  companion object {
    /// **What was brought in from a page** (`Pages.bring`), waiting for the app to take it: the
    /// selection's HTML and words, or the page itself, with the page's exact address and title, and
    /// the note it goes into.
    val brought = mutableListOf<JSONObject>()
    var current: java.lang.ref.WeakReference<MainActivity>? = null

    fun bring(piece: JSONObject) {
      synchronized(brought) { brought.add(piece) }
      val a = current?.get() ?: return
      a.runOnUiThread { a.wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null) }
    }

    /// The most one shared file may be: it travels to the page as base64 text.
    private const val MAX_SHARE = 25L * 1024 * 1024
    /// The most of a page's HTML handed over to be saved (characters).
    private const val MAX_PAGE = 5_000_000
    /// The selection bar's *Bring into note*.
    private const val BRING_ITEM = 0x666d
    /// What a page gives when brought in: its selection, or itself.
    private const val PICK = """
      (function () {
        var s = window.getSelection(), t = String(s || '').trim(), h = '';
        if (t && s.rangeCount) {
          var d = document.createElement('div');
          d.appendChild(s.getRangeAt(0).cloneContents());
          h = d.innerHTML;
        }
        return JSON.stringify({ kind: t ? 'selection' : 'page', text: t, html: h,
          pageUrl: location.href, pageTitle: document.title });
      })()
    """
    /// A passage brought in is text a person chose; a whole book pasted into Share is cut here.
    private const val MAX_TEXT = 100_000
    /// The newest release's APK — the address `docs/src/user/android.md` gives for the first install.
    private const val LATEST_APK =
      "https://github.com/formicaria-org/formicaria/releases/latest/download/formicaria-android.apk"

    /// Reads the interface and writes the four properties `app.css` consumes. Defensive because it
    /// also runs at document start, where a hostile-looking absence is simply "not injected yet".
    private const val APPLY_INSETS = """
      (function () {
        try {
          var i = JSON.parse(window.__fmInsets.json());
          var s = document.documentElement.style;
          s.setProperty('--safe-top', i.top + 'px');
          s.setProperty('--safe-right', i.right + 'px');
          s.setProperty('--safe-bottom', i.bottom + 'px');
          s.setProperty('--safe-left', i.left + 'px');
          // The keyboard: written only when it changes, so a rotation does not re-fire the event.
          var kb = (i.keyboard || 0) + 'px';
          if (s.getPropertyValue('--kb') !== kb) {
            s.setProperty('--kb', kb);
            if (i.keyboard > 0) document.documentElement.setAttribute('data-keyboard', '');
            else document.documentElement.removeAttribute('data-keyboard');
            window.dispatchEvent(new Event('fm-keyboard'));
          }
        } catch (e) {}
      })();
    """
  }
}
KT

# 2a) **The phone's page protections** (`decisions.md` 2026-11-17, *privacy switches*): what the
#     Privacy sheet's switches do in the page views. Plain functions over URLs, plus the scripts a page
#     gets at its start; `MainActivity` calls them. Ad and tracker blocking lives in `Blocker`.
cat > "$pkg_dir/PagePrivacy.kt" <<'KT'
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
  @Volatile var fired = -1L
  @Volatile var sitesOff: List<String> = emptyList()
  /** The cookie pop-up script (`ui/page/consent.ts`), handed over by the app. */
  @Volatile var consentScript: String? = null
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

  /** The address without its tracking parameters, or null when it had none. */
  fun clean(url: String): String? {
    if (!cleanLinks || off(url)) return null
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
  @JvmStatic external fun configure(trackers: Boolean, ads: Boolean)
  @JvmStatic external fun check(url: String, page: String, kind: String): Boolean
  @JvmStatic external fun hide(url: String): String
  @JvmStatic external fun hideNames(url: String, classes: String, ids: String): String
  @JvmStatic external fun hostBlocked(host: String): Boolean

  /** A request's type as the lists name them, from what the page view knows of it. */
  fun kind(url: String, accept: String?, mainFrame: Boolean): String {
    if (mainFrame) return "document"
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
KT


# 2a') **Save media on the phone** (`decisions.md` 2026-11-17): yt-dlp, Python and ffmpeg inside the
#      app (youtubedl-android, GPL-3.0: the phone APK is distributed under GPL-3.0 because of it; the
#      source stays MIT). The shell runs it; the core says what to ask and files the result.
cat > "$pkg_dir/MediaRun.kt" <<'KT'
package dev.formicaria.notes

import android.content.Context
import com.yausername.ffmpeg.FFmpeg
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.ConcurrentHashMap

class MediaJob(val id: String) {
  @Volatile var state = "tools"
  @Volatile var progress = -1.0
  @Volatile var title = ""
  @Volatile var file = ""
  @Volatile var info = "null"
  @Volatile var probe = ""
  @Volatile var error = ""
  fun json(): JSONObject = JSONObject().put("id", id).put("state", state)
    .put("progress", if (progress < 0) JSONObject.NULL else progress).put("title", title).put("file", file)
    .put("info", info).put("probe", probe).put("error", error)
}

object MediaRun {
  private val jobs = ConcurrentHashMap<String, MediaJob>()
  @Volatile private var ready = false

  /** The tools unpacked (once, the first time) and yt-dlp kept current (weekly, by its own updater). */
  private fun ensure(ctx: Context) {
    synchronized(this) {
      if (!ready) {
        YoutubeDL.getInstance().init(ctx)
        FFmpeg.getInstance().init(ctx)
        ready = true
      }
    }
    freshen(ctx, false)
  }

  fun freshen(ctx: Context, force: Boolean) {
    val prefs = ctx.getSharedPreferences("media", Context.MODE_PRIVATE)
    val last = prefs.getLong("checked", 0L)
    if (!force && System.currentTimeMillis() - last < 7L * 24 * 3600 * 1000) return
    try { YoutubeDL.getInstance().updateYoutubeDL(ctx, YoutubeDL.UpdateChannel._STABLE) } catch (e: Throwable) { }
    prefs.edit().putLong("checked", System.currentTimeMillis()).apply()
  }

  fun workDir(ctx: Context): File = File(ctx.cacheDir, "media").apply { mkdirs() }

  /** yt-dlp's `-J` for an address: what it holds, before anything is fetched. */
  fun probe(ctx: Context, id: String, url: String) {
    val job = MediaJob(id).also { jobs[id] = it }
    Thread {
      try {
        ensure(ctx)
        job.state = "probing"
        val r = YoutubeDLRequest(url)
        r.addOption("-J"); r.addOption("--no-playlist"); r.addOption("--no-warnings")
        job.probe = YoutubeDL.getInstance().execute(r, id, null).out
        job.state = "probed"
      } catch (e: Throwable) {
        job.error = e.message ?: "The media could not be looked at."
        job.state = "failed"
      }
    }.start()
  }

  /** Fetch for a choice: `args` are the core's (`fm_app::media::ytdlp_args`), the address last. */
  fun start(ctx: Context, id: String, url: String, argsJson: String) {
    val job = MediaJob(id).also { jobs[id] = it }
    Thread {
      try {
        ensure(ctx)
        job.state = "downloading"
        val tokens = JSONArray(argsJson).let { a -> (0 until a.length()).map { a.getString(it) } }
        val r = YoutubeDLRequest(url)
        var i = 0
        while (i < tokens.size) {
          val t = tokens[i]
          if (t == "--") break
          val next = tokens.getOrNull(i + 1)
          if (next != null && !next.startsWith("-")) { r.addOption(t, next); i += 2 } else { r.addOption(t); i += 1 }
        }
        val resp = YoutubeDL.getInstance().execute(r, id) { _, _, line -> read(job, line) }
        resp.out.lines().forEach { read(job, it) }
        if (job.state == "cancelled") return@Thread
        if (job.file.isEmpty() || !File(job.file).exists()) throw IllegalStateException("yt-dlp finished without a file")
        job.state = "fetched"
      } catch (e: Throwable) {
        if (job.state != "cancelled") {
          job.error = e.message ?: "The media could not be fetched."
          job.state = "failed"
        }
      }
    }.start()
  }

  private fun read(job: MediaJob, line: String) {
    val l = line.trim()
    when {
      l.startsWith("FMPROG ") -> {
        val n = l.removePrefix("FMPROG ").split(" ").map { it.toDoubleOrNull() }
        val done = n.getOrNull(0); val total = n.getOrNull(1) ?: n.getOrNull(2)
        if (done != null && total != null && total > 0) job.progress = (done / total).coerceIn(0.0, 1.0)
      }
      l.startsWith("FMFILE ") -> job.file = l.removePrefix("FMFILE ")
      l.startsWith("FMINFO ") -> { job.info = l.removePrefix("FMINFO "); job.title = try { JSONObject(job.info).optString("title") } catch (e: Exception) { "" } }
      l.startsWith("[ExtractAudio]") || l.startsWith("[Merger]") -> { job.state = "converting"; job.progress = -1.0 }
    }
  }

  fun cancel(id: String) {
    jobs[id]?.state = "cancelled"
    try { YoutubeDL.getInstance().destroyProcessById(id) } catch (e: Throwable) { }
  }

  fun jobs(): String = JSONArray(jobs.values.map { it.json() }).toString()

  /** A job done with (filed, or its failure shown): its leftovers go. */
  fun forget(id: String) {
    val j = jobs.remove(id) ?: return
    if (j.file.isNotEmpty()) File(j.file).delete()
  }
}
KT

# 2a'') **Music plays on in the background** (`decisions.md` 2026-11-17; `music-plan-2026-11-13.md`
#       M4(b)): a Media3 player in a media service, holding the whole queue, so the lock screen, the
#       notification, headphones, Bluetooth and the car control it, and it goes on with the screen
#       off. The app's player (`player.svelte.ts`) is its remote and mirrors what it says.
cat > "$pkg_dir/PlaybackService.kt" <<'KT'
package dev.formicaria.notes

import android.net.Uri
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import java.io.File

class PlaybackService : MediaSessionService() {
  private var session: MediaSession? = null

  override fun onCreate() {
    super.onCreate()
    val p = ExoPlayer.Builder(this)
      .setAudioAttributes(AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(), true)
      .setHandleAudioBecomingNoisy(true)
      .build()
    player = p
    session = MediaSession.Builder(this, p).build()
    pending?.invoke(p)
    pending = null
  }

  override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

  /** Swiped away while paused: nothing is playing, so the service goes too. */
  override fun onTaskRemoved(rootIntent: android.content.Intent?) {
    val p = player
    if (p == null || !p.playWhenReady || p.mediaItemCount == 0) stopSelf()
  }

  override fun onDestroy() {
    session?.run { player.release(); release() }
    session = null
    player = null
    super.onDestroy()
  }

  companion object {
    @Volatile var player: ExoPlayer? = null
    /** What to do once the service has made its player. */
    @Volatile var pending: ((ExoPlayer) -> Unit)? = null

    fun item(path: String, title: String, artist: String, album: String, cover: String?): MediaItem {
      val md = MediaMetadata.Builder().setTitle(title).setArtist(artist).setAlbumTitle(album)
      if (cover != null && File(cover).exists()) md.setArtworkUri(Uri.fromFile(File(cover)))
      return MediaItem.Builder().setUri(Uri.fromFile(File(path))).setMediaMetadata(md.build()).build()
    }
  }
}
KT

# 2d) **Trackers in other apps refused, phone-wide** (`decisions.md` 2026-11-17): a local VPN that
#     handles only address lookups (DNS). A lookup of a tracker's address gets "no such address";
#     every other is passed to the network's own resolver. No traffic but lookups enters it, and
#     nothing leaves the phone that would not have. Which app asked for which tracker is counted.
cat > "$pkg_dir/TrackerVpn.kt" <<'KT'
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
KT

# 2b) **Do NOT try to patch anything under `generated/` from here — it will not survive.**
#
# Attempted 2026-07-31, for a good reason: `RustWebViewClient` overrides no `onRenderProcessGone`,
# and the framework default for that is to **kill the app process** — so on a memory-tight phone
# (a phone that swaps ~3 GB while LMKD reaps Chrome and the Play Store, formicaria surviving on this
# very service while holding ~2 GB of model mappings) the app can simply vanish with nothing
# anywhere saying why. A logging override would at least name the cause, since `didCrash()`
# distinguishes "the system took the renderer" from "the renderer crashed".
#
# **It was applied, and then silently overwritten**: `tauri android build` re-runs its own codegen
# over `generated/` *during* the build, i.e. after this pre-build hook has finished. The patch
# reappeared and vanished on every build and no error was raised anywhere — verified by grepping the
# file mid-build. There is no post-codegen, pre-compile hook to hang it on. `AgentService.kt` and
# `MainActivity.kt` are only editable here because they sit **outside** `generated/`.
#
# So the diagnostic is not built, and the gap is recorded in `docs/context/known-issues.md` instead
# of being pretended to. If it is ever worth having, it needs a Gradle source-transform (or an
# upstream tauri hook), not another awk block here.

# 3) Manifest: the foreground-service permissions (after INTERNET) and the <service> (before
#    </application>). Idempotent — only inject what is absent.
if ! grep -q 'FOREGROUND_SERVICE"' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !p {
            print
            print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE\" />"
            print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_DATA_SYNC\" />"
            print "    <uses-permission android:name=\"android.permission.POST_NOTIFICATIONS\" />"
            p=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
# Microphone: in-app audio recording is a plain getUserMedia in the WebView, and wry's
# onPermissionRequest already asks for RECORD_AUDIO + MODIFY_AUDIO_SETTINGS at runtime and grants the
# WebView's AUDIO_CAPTURE on approval — but Android only lets it *request* a permission that is
# declared here. Declaring these makes in-app recording work on the phone with the same web code the
# browser build uses (any-device by construction). Idempotent.
if ! grep -q 'RECORD_AUDIO' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !a {
            print
            print "    <uses-permission android:name=\"android.permission.RECORD_AUDIO\" />"
            print "    <uses-permission android:name=\"android.permission.MODIFY_AUDIO_SETTINGS\" />"
            a=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
# No update installer (`decisions.md#track-m`, 2026-10-31, *a phone update opens in the browser*):
# `REQUEST_INSTALL_PACKAGES` is not declared, because Update now opens the newest APK in the
# browser and Android's own installer takes it from there. A manifest written by an earlier run of
# this script still carries the line, so it is taken out here.
sed -i '/android.permission.REQUEST_INSTALL_PACKAGES/d' "$manifest"
# **The camera must be visible to the app, or "take a photo" opens the gallery** (owner, 2026-10-18:
# food pictures could only come from the gallery). wry opens the camera only after asking Android
# whether any app answers `ACTION_IMAGE_CAPTURE` (`RustWebChromeClient.kt`, `resolveActivity`), and
# since Android 11 an app that targets API 30+ sees only the apps it declares it will talk to — so
# the answer was "none", and wry fell back to the file picker without a word. This `<queries>`
# block is that declaration, for photos and videos; it grants nothing and asks for no permission.
# It sits beside `<application>`, as the manifest requires. Idempotent.
if ! grep -q 'IMAGE_CAPTURE' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<application/ && !q {
            print "    <queries>"
            print "        <intent>"
            print "            <action android:name=\"android.media.action.IMAGE_CAPTURE\" />"
            print "        </intent>"
            print "        <intent>"
            print "            <action android:name=\"android.media.action.VIDEO_CAPTURE\" />"
            print "        </intent>"
            print "    </queries>"
            q=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
if ! grep -q 'AgentService' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/application>/ && !s {
            print "        <service"
            print "            android:name=\".AgentService\""
            print "            android:exported=\"false\""
            print "            android:foregroundServiceType=\"dataSync\" />"
            s=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# No reminders on Android (2026-10-31, see 1b): no `RECEIVE_BOOT_COMPLETED` and no receivers. A
# manifest written by an earlier run still carries them, so they are taken out here.
sed -i '/android.permission.RECEIVE_BOOT_COMPLETED/d; /<receiver android:name=".ReminderReceiver"/d' "$manifest"
sed -i '/<receiver android:name=".BootReceiver"/,/<\/receiver>/d' "$manifest"

# Share: formicaria in Android's share menu for pictures (`decisions.md#ui`, 2026-10-20). One file
# or several; the activity takes them (`takeShared`). Pictures only: what a screenshot is, and what
# the note editor draws. Idempotent.
if ! grep -q 'android.intent.action.SEND"' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/activity>/ && !sh {
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"image/*\" />"
            print "            </intent-filter>"
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND_MULTIPLE\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"image/*\" />"
            print "            </intent-filter>"
            sh=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# Bring in (2026-11-16): formicaria in Share for text (a page's link, a passage), and in **the
# text-selection menu of every app** (`PROCESS_TEXT`, labelled with the app's name). Separate from the
# pictures' block, so a manifest written before still gets it. Idempotent.
if ! grep -q 'android.intent.action.PROCESS_TEXT' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/activity>/ && !tx {
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"text/plain\" />"
            print "            </intent-filter>"
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.PROCESS_TEXT\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"text/plain\" />"
            print "            </intent-filter>"
            tx=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **Save media's tools** (2026-11-17): youtubedl-android and its ffmpeg, in the generated Gradle file
# (regenerated by `tauri android init`, so patched here, idempotently). Only the architecture being
# built (`abiList`, which Tauri passes; arm64 by default): the tools' native files are large.
gradle="$root/mobile/src-tauri/gen/android/app/build.gradle.kts"
if [ -f "$gradle" ] && ! grep -q 'youtubedl-android' "$gradle"; then
    tmpf=$(mktemp)
    awk '
        /^dependencies \{/ && !d { print; print "    implementation(\"io.github.junkfood02.youtubedl-android:library:0.18.1\")"; print "    implementation(\"io.github.junkfood02.youtubedl-android:ffmpeg:0.18.1\")"; d=1; next }
        /defaultConfig \{/ && !n { print; print "        ndk { abiFilters += ((project.findProperty(\"abiList\") as String?) ?: \"arm64-v8a\").split(\",\") }"; n=1; next }
        { print }
    ' "$gradle" > "$tmpf" && mv "$tmpf" "$gradle"
fi

# The native player's library (Media3), its own step so a Gradle file patched before still gets it.
# 1.6.1: the newest whose Kotlin metadata this project's Kotlin (1.9) reads; 1.11 needs Kotlin 2.2.
if [ -f "$gradle" ] && ! grep -q 'media3-session' "$gradle"; then
    tmpf=$(mktemp)
    awk '
        /^dependencies \{/ && !d { print; print "    implementation(\"androidx.media3:media3-exoplayer:1.6.1\")"; print "    implementation(\"androidx.media3:media3-session:1.6.1\")"; d=1; next }
        { print }
    ' "$gradle" > "$tmpf" && mv "$tmpf" "$gradle"
fi

# **The tracker block's service** (2026-11-17, in the one APK for everyone): Android binds it only
# with the VPN permission the system holds; it asks the person before the first start. Idempotent.
if ! grep -q 'TrackerVpn' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/application>/ && !v {
            print "        <service android:name=\".TrackerVpn\" android:permission=\"android.permission.BIND_VPN_SERVICE\" android:exported=\"false\">"
            print "            <intent-filter><action android:name=\"android.net.VpnService\" /></intent-filter>"
            print "        </service>"
            v=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **The media service** (background playback, 2026-11-17): declared with its type and permission
# (`ci/checks.sh` allows exactly this one beyond the earlier list). Idempotent.
if ! grep -q 'PlaybackService' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<application/ && !p { print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK\" />"; p=1 }
        /<\/application>/ && !s {
            print "        <service android:name=\".PlaybackService\" android:foregroundServiceType=\"mediaPlayback\" android:exported=\"true\">"
            print "            <intent-filter><action android:name=\"androidx.media3.session.MediaSessionService\" /></intent-filter>"
            print "        </service>"
            s=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **Open with…** (2026-11-17): a content link to a vault file, for another app to play, through the
# FileProvider Tauri's template declares (`${applicationId}.fileprovider`): its paths gain the app's
# own storage (`files-path`). A grant is per file, per opening. Idempotent.
paths_xml="$(dirname "$manifest")/res/xml/file_paths.xml"
rm -f "$(dirname "$manifest")/res/xml/fm_files.xml"
if [ -f "$paths_xml" ] && ! grep -q 'files-path' "$paths_xml"; then
    sed -i 's#</paths>#  <files-path name="vault" path="." />\n</paths>#' "$paths_xml"
fi

echo "android-inject-service: foreground service and update download injected (AgentService + MainActivity + manifest)."
