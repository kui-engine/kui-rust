// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_media_lottie.rs; edit that file, not this copy.
//! API tour: game audio, media elements and Lottie (examples/api_tours/media_lottie). At start-up
//! the host drives the document's audio manager (play by path into groups, per-handle volume /
//! pan / pitch / pause / seek, group volume and mute, pause-all / stop-all, preload / unload,
//! counts), an <audio> and a <video> element (src, load, loop / autoplay / muted / volume flags,
//! seek, play / pause, network and ready state) and a <lottie-player> (state, natural size, speed,
//! loop, pause, seek, stop), checking every call against the state read back. Time-dependent
//! results are read once frames have run; #tour-result ends as "PASS n/n". Works without an audio
//! device (KUI_MEDIA_NO_DEVICE=1): the checks read state, they do not listen.
//!
//! Binding note: AudioGroup is a Rust enum, so the C/Go check that reads an out-of-range group
//! (KUI_ERR_INVALID_ARG) cannot be expressed here.

use crate::canonical_runner::Context;
use kui::sys::{KuiStatus, KUI_ERR_INVALID_ARG, KUI_ERR_IO, KUI_ERR_NOT_SUPPORTED};
use kui::{
    AudioGroup, AudioPlayOptions, Document, Element, LottieLoadState, LottieState, Platform,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug). A failure is
//            logged as "KNOWN BUG" and kept out of the verdict; once it passes it counts as ok.

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    known: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            known: Cell::new(0),
            failed_names: RefCell::new(Vec::new()),
        }
    }

    fn row(&self, name: &str, result: &str, cell: &str) {
        let Ok(Some(log)) = self.doc.get_element_by_id("tour-log") else {
            return;
        };
        let Ok(row) = self.doc.create_element("tr") else {
            return;
        };
        let _ = row.set_attribute("data-check", name);
        let _ = row.set_attribute("data-result", result);
        if result != "ok" {
            let _ = row.class_list_add("bad");
        }
        for text in [name, cell] {
            if let Ok(td) = self.doc.create_element("td") {
                let _ = td.set_text_content(text);
                let _ = row.append_child(&td);
            }
        }
        let _ = log.append_child(&row);
    }

    fn check(&self, name: &str, ok: bool, detail: String) {
        self.total.set(self.total.get() + 1);
        if ok {
            self.row(name, "ok", "ok");
            return;
        }
        self.failed.set(self.failed.get() + 1);
        self.failed_names.borrow_mut().push(name.to_string());
        eprintln!("tour check failed: {name}: {detail}");
        self.row(name, "fail", &format!("FAIL: {detail}"));
    }

    fn known_bug(&self, name: &str, ok: bool, bug: &str, detail: String) {
        if ok {
            self.total.set(self.total.get() + 1);
            self.row(name, "ok", "ok");
            return;
        }
        self.known.set(self.known.get() + 1);
        eprintln!("tour known bug: {name}: {detail}");
        self.row(name, "known", &format!("KNOWN BUG: {bug} ({detail})"));
    }

    fn finish(&self) {
        let Ok(Some(result)) = self.doc.get_element_by_id("tour-result") else {
            return;
        };
        let (total, failed, known) = (self.total.get(), self.failed.get(), self.known.get());
        let mut text = if failed == 0 {
            format!("PASS {total}/{total}")
        } else {
            format!(
                "FAIL {failed}/{total}: {}",
                self.failed_names.borrow().join(", ")
            )
        };
        if known > 0 {
            text.push_str(&format!(" (known bugs: {known})"));
        }
        let _ = result.set_text_content(&text);
        let _ = result.set_attribute("data-total", &total.to_string());
        let _ = result.set_attribute("data-failed", &failed.to_string());
        let _ = result.set_attribute("data-known", &known.to_string());
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- Small helpers ----------------------------------------------------------------------

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// The KuiStatus of a result (0 for Ok).
fn status<T>(result: &kui::Result<T>) -> KuiStatus {
    result.as_ref().err().map_or(0, |e| e.status())
}

fn opts(group: AudioGroup, volume: f32, pan: f32, pitch: f32, looping: bool) -> AudioPlayOptions {
    AudioPlayOptions {
        group,
        volume,
        pan,
        pitch,
        looping,
        preload_only: false,
    }
}

const AUDIO_SEEK: f64 = 300.0; // far from where playback is after a second (the file is 675 s)
const VIDEO_SEEK: f64 = 5.0; // the clip is 13.3 s

struct Host {
    doc: Document,
    platform: Rc<Platform>,
    t: Tour,
    mp3: String,
    mp4: String,
    missing: String,
    available: Cell<bool>,
    music: Cell<u32>, // the looping music handle the buttons drive
    lottie_mark: Cell<f64>,
    au_paused: Cell<bool>,
    au_quiet: Cell<bool>,
    au_right: Cell<bool>,
    au_fast: Cell<bool>,
    au_all_paused: Cell<bool>,
    au_muted: Cell<bool>,
    lot_fast: Cell<bool>,
    vid_controls: Cell<bool>,
}

impl Host {
    fn el(&self, id: &str) -> Option<Element<'_>> {
        self.doc.get_element_by_id(id).ok().flatten()
    }

    fn set_text(&self, id: &str, text: &str) {
        if let Some(e) = self.el(id) {
            let _ = e.set_text_content(text);
        }
    }

    fn active(&self, group: Option<AudioGroup>) -> u32 {
        self.doc
            .audio()
            .counts(group)
            .map_or(0, |(active, _)| active)
    }

    fn pooled(&self) -> u32 {
        self.doc
            .audio()
            .counts(None)
            .map_or(0, |(_, pooled)| pooled)
    }

    // ---- Game audio: availability, one looping music sound, sfx, groups, pool -----------

    fn check_audio_groups(&self) {
        let audio = self.doc.audio();
        // Group volume / mute live in the manager and work with or without a device.
        let _ = audio.set_group(AudioGroup::Music, 0.4, true);
        let (volume, muted) = audio.group(AudioGroup::Music).unwrap_or((-1.0, false));
        let bad_volume = audio.set_group(AudioGroup::Music, 2.0, false);
        let _ = audio.set_group(AudioGroup::Music, 1.0, false);
        let (restored, restored_muted) = audio.group(AudioGroup::Music).unwrap_or((-1.0, true));
        self.t.check(
            "audio.set_group",
            near(volume.into(), 0.4, 0.001)
                && muted
                && status(&bad_volume) == KUI_ERR_INVALID_ARG
                && near(restored.into(), 1.0, 0.001)
                && !restored_muted,
            format!("music 0.4/muted -> {volume:.2}/{muted}, volume 2 {bad_volume:?}, restored {restored:.2}/{restored_muted}"),
        );
    }

    fn check_audio_unavailable(&self) {
        let audio = self.doc.audio();
        let played = audio.play(&self.mp3, &AudioPlayOptions::default());
        let loaded = audio.preload(&self.mp3, 2).unwrap_or(9);
        self.t.check(
            "audio.play-unavailable",
            status(&played) == KUI_ERR_NOT_SUPPORTED && loaded == 0,
            format!("play {played:?}, preload loaded {loaded}"),
        );
    }

    fn check_audio_handles(&self) {
        let audio = self.doc.audio();
        let loaded = audio.preload(&self.mp3, 2);
        let pooled = self.pooled();
        self.t.check(
            "audio.preload",
            matches!(loaded, Ok(2)) && pooled >= 2,
            format!("{loaded:?} pooled {pooled}"),
        );

        let played = audio.play(&self.mp3, &opts(AudioGroup::Music, 0.5, -0.5, 1.25, true));
        let music = *played.as_ref().unwrap_or(&0);
        self.music.set(music);
        let info = audio.info(music).ok().flatten();
        self.t.check(
            "audio.play",
            music != 0
                && info.is_some_and(|i| {
                    i.group == AudioGroup::Music
                        && near(i.volume.into(), 0.5, 0.001)
                        && near(i.pan.into(), -0.5, 0.001)
                        && near(i.pitch.into(), 1.25, 0.001)
                        && i.looping
                        && i.playing
                }),
            format!("{played:?} info {info:?}"),
        );

        let _ = audio.set_volume(music, 0.3);
        let _ = audio.set_pan(music, 0.7);
        let _ = audio.set_pitch(music, 0.8);
        let info = audio.info(music).ok().flatten();
        self.t.check(
            "audio.set_volume_pan_pitch",
            info.is_some_and(|i| {
                near(i.volume.into(), 0.3, 0.001)
                    && near(i.pan.into(), 0.7, 0.001)
                    && near(i.pitch.into(), 0.8, 0.001)
            }),
            format!("info {info:?}"),
        );

        let _ = audio.set_paused(music, true);
        let paused = audio.info(music).ok().flatten().map(|i| i.playing);
        let _ = audio.set_paused(music, false);
        let resumed = audio.info(music).ok().flatten().map(|i| i.playing);
        self.t.check(
            "audio.set_paused",
            paused == Some(false) && resumed == Some(true),
            format!("playing while paused {paused:?}, after resume {resumed:?}"),
        );

        let seek = audio.seek(music, 1.5);
        let bad = audio.seek(music, -1.0);
        self.t.check(
            "audio.seek-args",
            seek.is_ok() && status(&bad) == KUI_ERR_INVALID_ARG,
            format!("seek {seek:?}, seek(-1) {bad:?}"),
        );

        let missing = audio.play(&self.missing, &AudioPlayOptions::default());
        self.t.check(
            "audio.play-missing-file",
            status(&missing) == KUI_ERR_IO,
            format!("{missing:?}"),
        );
    }

    fn check_audio_bulk(&self) {
        let audio = self.doc.audio();
        let sfx = opts(AudioGroup::Sfx, 0.2, 0.0, 1.0, false);
        let first = audio.play(&self.mp3, &sfx).unwrap_or(0);
        let _ = audio.play(&self.mp3, &sfx);
        let (music, sfx_active, all) = (
            self.active(Some(AudioGroup::Music)),
            self.active(Some(AudioGroup::Sfx)),
            self.active(None),
        );
        self.t.check(
            "audio.get_counts",
            music == 1 && sfx_active == 2 && all == 3,
            format!("music {music} sfx {sfx_active} all {all}"),
        );

        let playing = |handle: u32| audio.info(handle).ok().flatten().map(|i| i.playing);
        let _ = audio.set_all_paused(Some(AudioGroup::Music), true);
        let (music_playing, sfx_playing) = (playing(self.music.get()), playing(first));
        let _ = audio.set_all_paused(None, false);
        let resumed = playing(self.music.get());
        self.t.check(
            "audio.set_all_paused",
            music_playing == Some(false) && sfx_playing == Some(true) && resumed == Some(true),
            format!("music while group paused {music_playing:?}, sfx {sfx_playing:?}, music after resume all {resumed:?}"),
        );

        let _ = audio.stop_all(Some(AudioGroup::Sfx));
        let (sfx_active, music) = (
            self.active(Some(AudioGroup::Sfx)),
            self.active(Some(AudioGroup::Music)),
        );
        self.t.check(
            "audio.stop_all",
            sfx_active == 0 && music == 1,
            format!("sfx {sfx_active} music {music}"),
        );
    }

    /// Once frames have run: the seek landed and the position moves; then stop and unload.
    fn check_audio_late(&self) {
        let audio = self.doc.audio();
        let music = self.music.get();
        let info = audio.info(music).ok().flatten();
        self.t.check(
            "audio.get_info-time",
            info.is_some_and(|i| i.duration > 1.0 && i.current_time > 1.45),
            format!("info {info:?} (seeked to 1.50)"),
        );

        let _ = audio.stop(music);
        let after = audio.info(music);
        let music_active = self.active(Some(AudioGroup::Music));
        self.t.check(
            "audio.stop",
            (matches!(after, Ok(None)) || matches!(after, Ok(Some(i)) if !i.playing))
                && music_active == 0,
            format!("info {after:?}, music active {music_active}"),
        );
        let unknown = audio.stop(987_654);
        self.t.check(
            "audio.stop-unknown",
            unknown.is_ok(),
            format!("{unknown:?}"),
        );

        let before = self.pooled();
        let _ = audio.unload(&self.mp3);
        let after = self.pooled();
        let _ = audio.unload_all();
        let all = self.pooled();
        self.t.check(
            "audio.unload",
            before > 0 && after == 0 && all == 0,
            format!("pooled {before} -> {after} after unload(path), {all} after unload_all"),
        );
        self.music.set(0);
    }

    fn check_audio(&self) {
        let available = self.doc.audio().available();
        let on = *available.as_ref().unwrap_or(&false);
        self.available.set(on);
        // Recorded as is: with KUI_MEDIA_NO_DEVICE=1 the plugin runs without a device.
        self.t.check(
            "audio.is_available",
            available.is_ok(),
            format!("{available:?}"),
        );
        self.set_text("au-avail", &format!("available={}", i32::from(on)));
        self.check_audio_groups();
        if !on {
            self.check_audio_unavailable();
            return;
        }
        self.check_audio_handles();
        self.check_audio_bulk();
    }

    // ---- Media elements: flags, volume, mute, src / load, seek, play / pause -----------

    fn media_time(media: &Element<'_>) -> f64 {
        media.current_time().unwrap_or(-1.0)
    }

    fn media_paused(media: &Element<'_>) -> i32 {
        media.paused().map_or(-1, i32::from)
    }

    fn check_media_setup(&self, name: &str, id: &str, src: &str) {
        let Some(media) = self.el(id) else {
            return self
                .t
                .check(&format!("{name}.setup"), false, format!("no #{id}"));
        };
        let state = media.network_state();
        self.t.check(
            &format!("{name}.network_state-empty"),
            matches!(state, Ok(0)),
            format!("networkState {state:?} before src (expected 0 NETWORK_EMPTY)"),
        );

        let _ = media.set_media_loop(true);
        let _ = media.set_media_autoplay(false);
        let flags = media.media_playback_flags();
        self.t.check(
            &format!("{name}.playback_flags"),
            matches!(flags, Ok((true, false, false))),
            format!("(loop, autoplay, ended) {flags:?}"),
        );

        let _ = media.set_muted(true);
        let on = media.muted();
        let _ = media.set_muted(false);
        let off = media.muted();
        let _ = media.set_volume(0.4);
        let volume = media.volume().unwrap_or(-1.0);
        self.t.check(
            &format!("{name}.muted_volume"),
            matches!((&on, &off), (Ok(true), Ok(false))) && near(volume.into(), 0.4, 0.001),
            format!("muted on {on:?} off {off:?}, volume {volume:.2}"),
        );

        let available = media.media_is_available();
        let _ = media.set_src(src);
        let loaded = media.load_media();
        let state = media.network_state();
        self.t.check(
            &format!("{name}.load"),
            loaded.is_ok()
                && available.is_ok()
                && matches!(state, Ok(2))
                && Self::media_paused(&media) == 1
                && Self::media_time(&media) == 0.0,
            format!(
                "{loaded:?} networkState {state:?} (expected 2 LOADING) paused {} available {available:?}",
                Self::media_paused(&media)
            ),
        );
    }

    fn check_media_not_media(&self) {
        let Some(plain) = self.el("plain") else {
            return self
                .t
                .check("media.not_media_element", false, "no #plain".into());
        };
        let media = plain.paused();
        let lottie = plain.lottie_state();
        self.t.check(
            "media.not_media_element",
            status(&media) == KUI_ERR_NOT_SUPPORTED && status(&lottie) == KUI_ERR_NOT_SUPPORTED,
            format!("paused {media:?}, lottie_state {lottie:?}"),
        );
    }

    /// A second after load(): metadata should be there without playing (HTML load()).
    fn check_media_metadata(&self, name: &str, id: &str) {
        let Some(media) = self.el(id) else { return };
        let duration = media.duration().unwrap_or(-1.0);
        let ready = media.ready_state().unwrap_or(-1);
        self.t.known_bug(
            &format!("{name}.metadata-before-play"),
            duration > 0.0 && ready >= 1,
            "media load lazily on the first play(): load() leaves readyState 0 and duration 0",
            format!("duration {duration:.2} readyState {ready} a second after load()"),
        );
    }

    /// Play loads the file: duration, network / ready state; then seek while playing.
    fn check_media_play(&self, name: &str, id: &str, seek_to: f64) {
        let Some(media) = self.el(id) else { return };
        let _ = media.play();
        let duration = media.duration().unwrap_or(-1.0);
        let network = media.network_state().unwrap_or(-1);
        let ready = media.ready_state().unwrap_or(-1);
        let paused = Self::media_paused(&media);
        self.t.check(
            &format!("{name}.play"),
            paused == 0 && duration > seek_to + 1.0 && network == 1 && ready >= 1,
            format!(
                "paused {paused} duration {duration:.2} networkState {network} readyState {ready}"
            ),
        );
        let _ = media.set_current_time(seek_to);
        let after = Self::media_time(&media);
        self.t.check(
            &format!("{name}.set_current_time"),
            near(after, seek_to, 0.3),
            format!("currentTime {after:.2} right after seek to {seek_to:.2}"),
        );
    }

    /// A while after the seek: playback continues from there; pause holds it.
    fn check_media_playing(&self, name: &str, id: &str, seek_to: f64, seek_bug: bool) {
        let Some(media) = self.el(id) else { return };
        let now = Self::media_time(&media);
        let detail = format!("currentTime {now:.2}, seeked to {seek_to:.2}");
        let check_name = format!("{name}.current_time-after-seek");
        if seek_bug {
            self.t.known_bug(
                &check_name,
                now >= seek_to && now < seek_to + 60.0,
                "seeking an <audio> element never reaches the player: playback goes on from the old position and currentTime falls back to it",
                detail,
            );
        } else {
            self.t
                .check(&check_name, now >= seek_to && now < seek_to + 5.0, detail);
        }
        let _ = media.pause();
        let (paused, time) = (Self::media_paused(&media), Self::media_time(&media));
        self.t.check(
            &format!("{name}.pause"),
            paused == 1 && time > 0.0,
            format!("paused {paused} currentTime {time:.2}"),
        );
    }

    // ---- Lottie: load, state, speed / loop, pause / seek / stop ---------------------------

    fn lottie_state(&self) -> LottieState {
        self.el("lot")
            .and_then(|l| l.lottie_state().ok())
            .unwrap_or(LottieState {
                playing: false,
                looping: false,
                autoplay: false,
                speed: 0.0,
                position: -1.0,
                load_state: LottieLoadState::Idle,
            })
    }

    fn check_lottie_loaded(&self) {
        let Some(lot) = self.el("lot") else { return };
        let state = self.lottie_state();
        let size = lot.lottie_natural_size();
        self.t.check(
            "lottie.get_state-loaded",
            state.load_state == LottieLoadState::Complete
                && state.playing
                && near(state.speed.into(), 1.0, 0.001),
            format!("state {state:?}"),
        );
        self.t.check(
            "lottie.get_natural_size",
            size.as_ref().is_ok_and(|s| {
                near(s.width.into(), 200.0, 0.5) && near(s.height.into(), 200.0, 0.5)
            }),
            format!("{size:?} (pulse.json is 200x200)"),
        );
        self.lottie_mark.set(state.position);
    }

    fn check_lottie_controls(&self) {
        let Some(lot) = self.el("lot") else { return };
        let state = self.lottie_state();
        self.t.check(
            "lottie.position-advances",
            state.position != self.lottie_mark.get(),
            format!(
                "position {:.3} then {:.3}",
                self.lottie_mark.get(),
                state.position
            ),
        );

        let _ = lot.set_lottie_speed(2.0);
        let speed = self.lottie_state().speed;
        let bad = lot.set_lottie_speed(-1.0);
        self.t.check(
            "lottie.set_speed",
            near(speed.into(), 2.0, 0.001) && status(&bad) == KUI_ERR_INVALID_ARG,
            format!("speed {speed:.2}, speed -1 {bad:?}"),
        );
        let _ = lot.set_lottie_speed(1.0);

        let _ = lot.set_lottie_loop(false);
        let off = self.lottie_state().looping;
        let _ = lot.set_lottie_loop(true);
        let on = self.lottie_state().looping;
        self.t.check(
            "lottie.set_loop",
            !off && on,
            format!("loop off {off}, on {on}"),
        );

        let _ = lot.pause_lottie();
        let _ = lot.seek_lottie(0.5);
        let after = self.lottie_state();
        self.t.check(
            "lottie.pause_seek",
            !after.playing && near(after.position, 0.5, 0.02),
            format!("playing {} position {:.3}", after.playing, after.position),
        );
    }

    fn check_lottie_held(&self) {
        let Some(lot) = self.el("lot") else { return };
        let state = self.lottie_state();
        self.t.check(
            "lottie.paused-holds",
            !state.playing && near(state.position, 0.5, 0.02),
            format!(
                "playing {} position {:.3} after a while",
                state.playing, state.position
            ),
        );
        let _ = lot.stop_lottie();
        let stopped = self.lottie_state();
        self.t.check(
            "lottie.stop",
            !stopped.playing && near(stopped.position, 0.0, 0.001),
            format!(
                "playing {} position {:.3}",
                stopped.playing, stopped.position
            ),
        );
        let _ = lot.play_lottie(); // leave it playing for the panel
    }

    // ---- Phases: immediate calls, then timers for results that need frames -------------

    fn phase_three(&self) {
        self.check_media_playing("audio", "aud", AUDIO_SEEK, true);
        self.check_media_playing("video", "vid", VIDEO_SEEK, false);
        self.check_lottie_held();
        if self.available.get() {
            self.check_audio_late();
        }
        self.t.finish();
    }

    fn phase_two(self: &Rc<Self>) {
        self.check_media_metadata("audio", "aud");
        self.check_media_metadata("video", "vid");
        self.check_media_play("audio", "aud", AUDIO_SEEK);
        self.check_media_play("video", "vid", VIDEO_SEEK);
        self.check_lottie_controls();
        let h = self.clone();
        let _ = self.platform.set_timeout(700, move || h.phase_three());
    }

    fn phase_one(self: &Rc<Self>) {
        self.check_lottie_loaded();
        let h = self.clone();
        let _ = self.platform.set_timeout(900, move || h.phase_two());
    }

    // ---- Panel buttons ------------------------------------------------------------------

    fn report_audio(&self) {
        let audio = self.doc.audio();
        let music = self.music.get();
        match audio.info(music) {
            Ok(Some(i)) => self.set_text(
                "au-out",
                &format!(
                    "music #{music} playing={} vol={:.2} pan={:.2} pitch={:.2} loop={} t={:.1}/{:.1}",
                    i32::from(i.playing),
                    i.volume,
                    i.pan,
                    i.pitch,
                    i32::from(i.looping),
                    i.current_time,
                    i.duration
                ),
            ),
            other => self.set_text("au-out", &format!("music: none (handle {music}, {other:?})")),
        }
        let (volume, muted) = audio.group(AudioGroup::Music).unwrap_or((0.0, false));
        self.set_text(
            "au-counts",
            &format!(
                "active={} sfx={} pooled={} music-group={volume:.1}{}",
                self.active(None),
                self.active(Some(AudioGroup::Sfx)),
                self.pooled(),
                if muted { " muted" } else { "" }
            ),
        );
    }

    fn report_lottie(&self) {
        let s = self.lottie_state();
        let load = match s.load_state {
            LottieLoadState::Idle => 0,
            LottieLoadState::Loading => 1,
            LottieLoadState::Complete => 2,
            LottieLoadState::Error => 3,
        };
        self.set_text(
            "lot-out",
            &format!(
                "playing={} loop={} speed={:.1} position={:.2} load={load}",
                i32::from(s.playing),
                i32::from(s.looping),
                s.speed,
                s.position
            ),
        );
    }

    fn report_media(&self, out: &str, id: &str) {
        let Some(media) = self.el(id) else { return };
        let (looping, autoplay, ended) = media
            .media_playback_flags()
            .unwrap_or((false, false, false));
        let text = format!(
            "paused={} t={:.1}/{:.1} muted={} vol={:.1} loop={} autoplay={} ended={} net={} ready={}",
            Self::media_paused(&media),
            Self::media_time(&media),
            media.duration().unwrap_or(-1.0),
            media.muted().map_or(-1, i32::from),
            media.volume().unwrap_or(-1.0),
            i32::from(looping),
            i32::from(autoplay),
            i32::from(ended),
            media.network_state().unwrap_or(-1),
            media.ready_state().unwrap_or(-1)
        );
        self.set_text(out, &text);
    }

    fn button(&self, id: &str) {
        let audio = self.doc.audio();
        let music = self.music.get();
        let toggle = |cell: &Cell<bool>| {
            cell.set(!cell.get());
            cell.get()
        };
        match id {
            "btn-au-play" => {
                let _ = audio.stop(music);
                self.music.set(
                    audio
                        .play(&self.mp3, &opts(AudioGroup::Music, 1.0, 0.0, 1.0, true))
                        .unwrap_or(0),
                );
                for cell in [
                    &self.au_paused,
                    &self.au_quiet,
                    &self.au_right,
                    &self.au_fast,
                ] {
                    cell.set(false);
                }
            }
            "btn-au-pause" => {
                let _ = audio.set_paused(music, toggle(&self.au_paused));
            }
            "btn-au-volume" => {
                let _ = audio.set_volume(music, if toggle(&self.au_quiet) { 0.3 } else { 1.0 });
            }
            "btn-au-pan" => {
                let _ = audio.set_pan(music, if toggle(&self.au_right) { 1.0 } else { -1.0 });
            }
            "btn-au-pitch" => {
                let _ = audio.set_pitch(music, if toggle(&self.au_fast) { 1.5 } else { 1.0 });
            }
            "btn-au-seek" => {
                let _ = audio.seek(music, 2.0);
            }
            "btn-au-stop" => {
                let _ = audio.stop(music);
            }
            "btn-au-sfx" => {
                let sfx = opts(AudioGroup::Sfx, 0.3, 0.0, 1.0, false);
                let _ = audio.play(&self.mp3, &sfx);
                let _ = audio.play(&self.mp3, &sfx);
            }
            "btn-au-stopsfx" => {
                let _ = audio.stop_all(Some(AudioGroup::Sfx));
            }
            "btn-au-pauseall" => {
                let _ = audio.set_all_paused(None, toggle(&self.au_all_paused));
            }
            "btn-au-mute" => {
                let muted = toggle(&self.au_muted);
                let _ = audio.set_group(AudioGroup::Music, if muted { 0.5 } else { 1.0 }, muted);
            }
            "btn-au-preload" => {
                let _ = audio.preload(&self.mp3, 3);
            }
            "btn-au-unload" => {
                let _ = audio.unload_all();
            }
            _ => {}
        }
        if id.starts_with("btn-au-") {
            self.report_audio();
            return;
        }
        if let Some(lot) = id.starts_with("btn-lot-").then(|| self.el("lot")).flatten() {
            let _ = match id {
                "btn-lot-play" => lot.play_lottie(),
                "btn-lot-pause" => lot.pause_lottie(),
                "btn-lot-seek0" => lot.seek_lottie(0.0),
                "btn-lot-seek50" => lot.seek_lottie(0.5),
                "btn-lot-stop" => lot.stop_lottie(),
                "btn-lot-speed" => {
                    lot.set_lottie_speed(if toggle(&self.lot_fast) { 2.0 } else { 1.0 })
                }
                "btn-lot-loop" => lot.set_lottie_loop(!self.lottie_state().looping),
                _ => Ok(()),
            };
            self.report_lottie();
            return;
        }
        let (media_id, out) = if id.starts_with("btn-aud-") {
            ("aud", "aud-out")
        } else {
            ("vid", "vid-out")
        };
        let Some(media) = self.el(media_id) else {
            return;
        };
        let flags = media
            .media_playback_flags()
            .unwrap_or((false, false, false));
        let _ = match id {
            "btn-aud-play" | "btn-vid-play" => media.play(),
            "btn-aud-pause" | "btn-vid-pause" => media.pause(),
            "btn-aud-seek" => media.set_current_time(3.0),
            "btn-vid-seek" => media.set_current_time(1.0),
            "btn-aud-mute" => media.set_muted(!media.muted().unwrap_or(false)),
            "btn-aud-loop" => media.set_media_loop(!flags.0),
            "btn-aud-load" => media.load_media(),
            "btn-vid-controls" => media.set_controls(toggle(&self.vid_controls)),
            "btn-vid-autoplay" => media.set_media_autoplay(!flags.1),
            _ => Ok(()),
        };
        self.report_media(out, media_id);
    }
}

const BUTTONS: [&str; 31] = [
    "btn-au-play",
    "btn-au-pause",
    "btn-au-volume",
    "btn-au-pan",
    "btn-au-pitch",
    "btn-au-seek",
    "btn-au-stop",
    "btn-au-sfx",
    "btn-au-stopsfx",
    "btn-au-pauseall",
    "btn-au-mute",
    "btn-au-preload",
    "btn-au-unload",
    "btn-lot-play",
    "btn-lot-pause",
    "btn-lot-seek0",
    "btn-lot-seek50",
    "btn-lot-stop",
    "btn-lot-speed",
    "btn-lot-loop",
    "btn-aud-play",
    "btn-aud-pause",
    "btn-aud-seek",
    "btn-aud-mute",
    "btn-aud-loop",
    "btn-aud-load",
    "btn-vid-play",
    "btn-vid-pause",
    "btn-vid-seek",
    "btn-vid-controls",
    "btn-vid-autoplay",
];

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let assets = exe
        .parent()
        .ok_or("executable has no directory")?
        .join("assets");
    let path = |parts: &[&str]| {
        parts
            .iter()
            .fold(assets.clone(), |p, part| p.join(part))
            .to_string_lossy()
            .into_owned()
    };
    let h = Rc::new(Host {
        doc: ctx.document.clone(),
        platform: ctx.platform.clone(),
        t: Tour::new(ctx.document),
        mp3: path(&["native_media", "sample.mp3"]),
        mp4: path(&["native_media", "sample.mp4"]),
        missing: path(&["native_media", "no_such_sound.mp3"]),
        available: Cell::new(false),
        music: Cell::new(0),
        lottie_mark: Cell::new(0.0),
        au_paused: Cell::new(false),
        au_quiet: Cell::new(false),
        au_right: Cell::new(false),
        au_fast: Cell::new(false),
        au_all_paused: Cell::new(false),
        au_muted: Cell::new(false),
        lot_fast: Cell::new(false),
        vid_controls: Cell::new(false),
    });

    let lot = ctx
        .document
        .get_element_by_id("lot")?
        .ok_or("page has no #lot")?;
    lot.set_lottie_src(&path(&["native_animation", "pulse.json"]))?;
    lot.set_lottie_loop(true)?;
    lot.play_lottie()?;

    h.check_audio();
    h.check_media_setup("audio", "aud", &h.mp3);
    h.check_media_setup("video", "vid", &h.mp4);
    h.check_media_not_media();
    let first = h.clone();
    ctx.platform.set_timeout(300, move || first.phase_one())?;

    for id in BUTTONS {
        let element = ctx
            .document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?;
        let handler = h.clone();
        element.add_event_listener("click", move |_| handler.button(id))?;
    }
    Ok(())
}
