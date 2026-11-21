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
    // **One song the player cannot read does not stop the rest** (2026-11-21, found in the
    // emulator: a malformed Ogg left the player in its error state, and nothing played again until
    // the app restarted). The next song is played; at the end of the list it is made ready for the
    // next press instead.
    p.addListener(object : Player.Listener {
      override fun onPlayerError(error: androidx.media3.common.PlaybackException) {
        if (p.hasNextMediaItem()) {
          p.seekToNextMediaItem()
          p.prepare()
          p.play()
        } else {
          p.prepare()
        }
      }
    })
    session = MediaSession.Builder(this, p).build()
    // Everything asked before the player existed, in the order asked.
    val asked = synchronized(pending) { pending.toList().also { pending.clear() } }
    asked.forEach { it(p) }
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
    /** What to do once the service has made its player, **every** call in order: pressing play
     *  asks for the queue, then the modes, then the volume, before the service is up — held in one
     *  slot, each replaced the one before, and the queue was never given (2026-11-20, found in the
     *  emulator: the first play after a start played nothing). */
    val pending = mutableListOf<(ExoPlayer) -> Unit>()

    fun item(path: String, title: String, artist: String, album: String, cover: String?): MediaItem {
      val md = MediaMetadata.Builder().setTitle(title).setArtist(artist).setAlbumTitle(album)
      if (cover != null && File(cover).exists()) md.setArtworkUri(Uri.fromFile(File(cover)))
      return MediaItem.Builder().setUri(Uri.fromFile(File(path))).setMediaMetadata(md.build()).build()
    }
  }
}
