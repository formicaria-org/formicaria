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
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

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

  // **Hand the window insets to the page — a platform fact only the platform has.**
  //
  // `env(safe-area-inset-*)` is 0 in wry's WebView: `viewport-fit=cover` is set, Android 15
  // forces edge-to-edge, and wry never forwards `WindowInsetsCompat` into the page. So the
  // stylesheet had been guessing — 1.75rem at the top, which is *less than this phone's camera
  // cutout*, and 0.5rem at the bottom, which clears a gesture pill but not a three-button
  // navigation bar. Both were visible on the owner's device and invisible to every test.
  //
  // We set the same custom properties `app.css` defines, as an inline style on the root element,
  // which outranks the `:root` rule. The floors stay there as a fallback: this fires on an event,
  // and a reload before it fires would paint under the camera again.
  override fun onWebViewCreate(webView: WebView) {
    wv = webView
    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, insets ->
      val i = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val d = v.resources.displayMetrics.density
      val js = "(function(){var s=document.documentElement.style;" +
        "s.setProperty('--safe-top','" + (i.top / d) + "px');" +
        "s.setProperty('--safe-right','" + (i.right / d) + "px');" +
        "s.setProperty('--safe-bottom','" + (i.bottom / d) + "px');" +
        "s.setProperty('--safe-left','" + (i.left / d) + "px');})()"
      v.post { (v as WebView).evaluateJavascript(js, null) }
      insets
    }
  }

  // A rotation, a keyboard, or coming back to the app can all change the insets; asking for them
  // again is cheap and re-runs the listener above.
  override fun onResume() {
    super.onResume()
    wv?.let { ViewCompat.requestApplyInsets(it) }
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
