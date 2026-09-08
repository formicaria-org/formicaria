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

# 2) MainActivity starts the service on launch. Overwrite the whole file (small, stable, and tauri
#    does not regenerate it — it is outside generated/).
cat > "$pkg_dir/MainActivity.kt" <<'KT'
package dev.formicaria.notes

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature

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
  }

  /// The insets the window last reported, in CSS pixels. Held so the page can **ask** for them.
  // `Float`, because `displayMetrics.density` is one and the division follows it.
  private var top = 0f
  private var right = 0f
  private var bottom = 0f
  private var left = 0f

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
      return "{\"top\":$t,\"right\":$right,\"bottom\":$bottom,\"left\":$left}"
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    wv = webView
    webView.addJavascriptInterface(Insets(), "__fmInsets")

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

echo "android-inject-service: foreground service injected (AgentService + MainActivity + manifest)."
