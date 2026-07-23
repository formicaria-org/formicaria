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
        return START_STICKY
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
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Keep the app + its network alive for the study assistant's download and inference.
    val svc = Intent(this, AgentService::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) startForegroundService(svc) else startService(svc)
  }
}
KT

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
