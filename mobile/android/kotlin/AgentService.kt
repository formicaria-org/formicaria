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
