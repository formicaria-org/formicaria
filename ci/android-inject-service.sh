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
        // owner's phone it was outliving Chrome and the Play Store). MainActivity starts this
        // service on every launch, so stickiness bought nothing a tap does not.
        return START_NOT_STICKY
    }
}
KT

# 1b) Reminders (`decisions.md#ui`, 2026-10-13, *a reminder is the note's own date, said at its
#     time*). The page hands over the moments; these set one alarm each, post the notification
#     when it fires, and set them again after the phone restarts. Overwritten each run.
cat > "$pkg_dir/Reminders.kt" <<'KT'
package dev.formicaria.notes

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import org.json.JSONArray

/// The alarms behind formicaria's reminders. **The list is the page's**: every call replaces what
/// was set before, so a note whose date changed or that was finished loses its alarm.
object Reminders {
  private const val PREFS = "fm-reminders"
  private const val KEY = "list"
  private const val CHANNEL = "reminders"
  /// No more than this many alarms are held at once; the next open sets the rest.
  private const val MOST = 50

  private fun flags(): Int =
    PendingIntent.FLAG_UPDATE_CURRENT or
      (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0)

  private fun intentFor(ctx: Context, slot: Int): PendingIntent =
    PendingIntent.getBroadcast(ctx, slot, Intent(ctx, ReminderReceiver::class.java), flags())

  /// Set the alarms in `json`: `[{"title": …, "text": …, "at": <ms since 1970>}]`. Returns how
  /// many were set. Kept, so `again` can set them after a restart.
  fun set(ctx: Context, json: String): Int {
    val alarms = ctx.getSystemService(Context.ALARM_SERVICE) as AlarmManager
    val prefs = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    // Every slot that may hold an old alarm is cleared first: the list is replaced, not added to.
    for (slot in 0 until MOST) alarms.cancel(intentFor(ctx, slot))
    val list = try { JSONArray(json) } catch (e: Exception) { JSONArray() }
    val now = System.currentTimeMillis()
    var set = 0
    for (i in 0 until list.length()) {
      if (set >= MOST) break
      val r = list.optJSONObject(i) ?: continue
      val at = r.optLong("at", 0L)
      if (at <= now) continue
      val fire = Intent(ctx, ReminderReceiver::class.java)
        .putExtra("title", r.optString("title", "formicaria"))
        .putExtra("text", r.optString("text", ""))
        .putExtra("slot", set)
      val pending = PendingIntent.getBroadcast(ctx, set, fire, flags())
      // Inexact on purpose: an exact alarm needs a permission people are right to be wary of, and
      // a reminder a few minutes late is still a reminder.
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
        alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
      } else {
        alarms.set(AlarmManager.RTC_WAKEUP, at, pending)
      }
      set += 1
    }
    prefs.edit().putString(KEY, json).apply()
    return set
  }

  /// After the phone restarts, alarms are gone: set the kept list again.
  fun again(ctx: Context) {
    val json = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY, null) ?: return
    set(ctx, json)
  }

  fun show(ctx: Context, slot: Int, title: String, text: String) {
    val manager = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      manager.createNotificationChannel(
        NotificationChannel(CHANNEL, "Reminders", NotificationManager.IMPORTANCE_DEFAULT)
      )
    }
    val open = PendingIntent.getActivity(
      ctx, 0, Intent(ctx, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK), flags()
    )
    val builder =
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) Notification.Builder(ctx, CHANNEL)
      else Notification.Builder(ctx)
    val notification = builder
      .setContentTitle(title)
      .setContentText(text)
      .setSmallIcon(ctx.applicationInfo.icon)
      .setContentIntent(open)
      .setAutoCancel(true)
      .build()
    // A notification Android will not show (the person said no) is not an error to raise.
    try { manager.notify(1000 + slot, notification) } catch (e: Exception) { }
  }
}

/// An alarm fired. Not exported: only this app's own pending intents reach it.
class ReminderReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    Reminders.show(
      context,
      intent.getIntExtra("slot", 0),
      intent.getStringExtra("title") ?: "formicaria",
      intent.getStringExtra("text") ?: "",
    )
  }
}

/// The phone restarted. Exported, as a receiver of a system broadcast must be, and so it does
/// one thing whatever it is sent: set the kept alarms again.
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action == Intent.ACTION_BOOT_COMPLETED) Reminders.again(context)
  }
}
KT

# 2) MainActivity starts the service on launch. Overwrite the whole file (small, stable, and tauri
#    does not regenerate it — it is outside generated/).
cat > "$pkg_dir/MainActivity.kt" <<'KT'
package dev.formicaria.notes

import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.provider.Settings
import android.util.Base64
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

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
    super.onCreate(savedInstanceState)
    // Keep the app + its network alive for the study assistant's download and inference.
    val svc = Intent(this, AgentService::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) startForegroundService(svc) else startService(svc)
    takeShared(intent)
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

  private fun takeShared(intent: Intent?) {
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
    @JavascriptInterface
    fun pending(): String = synchronized(shared) {
      val files = JSONArray()
      for (f in shared) files.put(JSONObject().put("name", f.name).put("mime", f.mime))
      val no = JSONArray()
      for ((name, why) in refused) no.put(JSONObject().put("name", name).put("why", why))
      JSONObject().put("files", files).put("refused", no).toString()
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

  /// **Reminders** (`decisions.md#ui`, 2026-10-13). The page knows the moments; this sets the
  /// alarms and says whether Android will show a notification at all. `ask` opens Android's own
  /// question, once; the page asks `allowed` again afterwards.
  inner class Remind {
    @JavascriptInterface
    fun set(json: String): Int = Reminders.set(applicationContext, json)

    @JavascriptInterface
    fun allowed(): Boolean {
      val manager = getSystemService(NOTIFICATION_SERVICE) as android.app.NotificationManager
      return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) manager.areNotificationsEnabled() else true
    }

    @JavascriptInterface
    fun ask() {
      if (Build.VERSION.SDK_INT >= 33) {
        runOnUiThread { requestPermissions(arrayOf("android.permission.POST_NOTIFICATIONS"), 7) }
      }
    }
  }

  /// **The one thing the page may ask Android to install: formicaria's own, already-checked update.**
  ///
  /// A phone cannot restart into a new version the way the desktop does — only Android's package
  /// installer can replace an app — so this is the last step of updating on Android, and it is
  /// deliberately narrow (`decisions.md#toolchain`, 2026-09-11):
  ///
  /// - **It takes no argument.** It installs one file: `cacheDir/update/formicaria.apk`, which the Rust
  ///   side writes only once its sha256 matches a manifest whose Ed25519 signature has been verified.
  ///   Nothing running in the page can name a different file, so a page that went wrong cannot turn
  ///   this into "install whatever I point at". `cacheDir` is also exactly what the existing
  ///   `FileProvider` shares (`cache-path "."`), so no resource needs editing — Tauri's
  ///   `app_cache_dir()` resolves to `activity.cacheDir`, checked against its `PathPlugin`.
  /// - **Android asks first, once.** Installing from an app needs the person's permission for that
  ///   app. Rather than fail, this opens the screen that grants it and says so; the page tells the
  ///   person to come back and press Install again.
  /// - **An older version is not handed over.** Android would refuse it with a sentence that reads
  ///   like a corrupt file; see `olderThanInstalled`.
  /// - **A download signed by somebody else is not offered.** Android enforces this itself for an
  ///   update, but only after the person has been shown an install prompt for a stranger's package.
  ///   The check here blocks only on a *positive* mismatch: a platform quirk that returns no signing
  ///   information must not block an update Android would accept — that enforcement stays Android's.
  ///
  /// Returns `ok`, `permission`, or `error: <reason>`; the page prints the reason as it is.
  inner class UpdateInstaller {
    @JavascriptInterface
    fun install(): String {
      val apk = File(cacheDir, "update/formicaria.apk")
      if (!apk.isFile) return "error: there is no checked download to install yet"
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !packageManager.canRequestPackageInstalls()) {
        val allow = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:$packageName"))
          .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        runOnUiThread { startActivity(allow) }
        return "permission"
      }
      if (signedDifferently(apk)) {
        return "error: the download is not signed like the formicaria already installed, so it was not installed"
      }
      if (olderThanInstalled(apk)) {
        return "error: the download is an older version than the formicaria on this phone. Android only installs the same version or a newer one, so it was not installed. Look for the newest version and get that."
      }
      return try {
        val uri = FileProvider.getUriForFile(this@MainActivity, "$packageName.fileprovider", apk)
        val view = Intent(Intent.ACTION_VIEW)
          .setDataAndType(uri, "application/vnd.android.package-archive")
          .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        runOnUiThread { startActivity(view) }
        "ok"
      } catch (e: Exception) {
        "error: ${e.message}"
      }
    }
  }

  /// True only when both version codes were read **and** the download's is lower.
  ///
  /// **Android refuses this itself, and says nothing useful when it does.** Handed an older package,
  /// its installer answers *"App not installed as package appears to be invalid"* — the same words
  /// it uses for a corrupt file — after the person has confirmed the install. The owner met that on
  /// 2026-10-11, when a test build was offered an older release. The updater no longer offers one
  /// (`fm_update::newer_than`), and this is the second line: if an older file ever reaches here,
  /// the person is told why in the app instead of being shown that sentence. Like the signer check
  /// it blocks only on a *positive* answer; what cannot be read is left to Android.
  private fun olderThanInstalled(apk: File): Boolean {
    return try {
      val theirs = packageManager.getPackageArchiveInfo(apk.path, 0)
      val mine = packageManager.getPackageInfo(packageName, 0)
      theirs != null && versionCodeOf(theirs) < versionCodeOf(mine)
    } catch (e: Exception) {
      false
    }
  }

  @Suppress("DEPRECATION")
  private fun versionCodeOf(p: android.content.pm.PackageInfo): Long =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) p.longVersionCode else p.versionCode.toLong()

  /// True only when both signer sets were read **and** they differ. See `UpdateInstaller`.
  private fun signedDifferently(apk: File): Boolean {
    return try {
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        @Suppress("DEPRECATION")
        val theirs = packageManager
          .getPackageArchiveInfo(apk.path, PackageManager.GET_SIGNING_CERTIFICATES)
          ?.signingInfo?.apkContentsSigners
        @Suppress("DEPRECATION")
        val mine = packageManager
          .getPackageInfo(packageName, PackageManager.GET_SIGNING_CERTIFICATES)
          .signingInfo?.apkContentsSigners
        theirs != null && mine != null && theirs.toSet() != mine.toSet()
      } else {
        @Suppress("DEPRECATION")
        val theirs = packageManager.getPackageArchiveInfo(apk.path, PackageManager.GET_SIGNATURES)?.signatures
        @Suppress("DEPRECATION")
        val mine = packageManager.getPackageInfo(packageName, PackageManager.GET_SIGNATURES).signatures
        theirs != null && mine != null && theirs.toSet() != mine.toSet()
      }
    } catch (e: Exception) {
      false
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    wv = webView
    webView.addJavascriptInterface(Insets(), "__fmInsets")
    // The installer bridge, re-injected into every document like the insets one. See UpdateInstaller.
    webView.addJavascriptInterface(UpdateInstaller(), "__fmUpdate")
    // Reminders: the page hands over the moments, Android sets the alarms. See Reminders.kt.
    webView.addJavascriptInterface(Remind(), "__fmRemind")
    // What another app shared: the page asks for it. See `takeShared`.
    webView.addJavascriptInterface(Share(), "__fmShare")

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
    /// The most one shared file may be: it travels to the page as base64 text.
    private const val MAX_SHARE = 25L * 1024 * 1024

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

# 2b) **Do NOT try to patch anything under `generated/` from here — it will not survive.**
#
# Attempted 2026-07-31, for a good reason: `RustWebViewClient` overrides no `onRenderProcessGone`,
# and the framework default for that is to **kill the app process** — so on a memory-tight phone
# (the owner's swaps ~3 GB while LMKD reaps Chrome and the Play Store, formicaria surviving on this
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
# The update installer: Android lets an app hand an APK to the system installer only if it declares
# this. **A deliberate widening, recorded in decisions.md** — people and scanners read it as a red
# flag, and it is what installing an update from inside the app costs. It grants no silent install:
# Android still asks the person, once for the permission and every time for the install. Idempotent.
if ! grep -q 'REQUEST_INSTALL_PACKAGES' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !r {
            print
            print "    <uses-permission android:name=\"android.permission.REQUEST_INSTALL_PACKAGES\" />"
            r=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
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

# Reminders: the permission to hear that the phone restarted, and the two receivers. Idempotent.
if ! grep -q 'RECEIVE_BOOT_COMPLETED' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !b {
            print
            print "    <uses-permission android:name=\"android.permission.RECEIVE_BOOT_COMPLETED\" />"
            b=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
if ! grep -q 'ReminderReceiver' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/application>/ && !r {
            print "        <receiver android:name=\".ReminderReceiver\" android:exported=\"false\" />"
            print "        <receiver android:name=\".BootReceiver\" android:exported=\"true\">"
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.BOOT_COMPLETED\" />"
            print "            </intent-filter>"
            print "        </receiver>"
            r=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

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

echo "android-inject-service: foreground service, update installer and reminders injected (AgentService + Reminders + MainActivity + manifest)."
