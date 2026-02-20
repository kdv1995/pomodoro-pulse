use chrono::{Datelike, Local, TimeZone, Utc};
use httparse::Status;
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_notification::NotificationExt;

const APP_SETTINGS_KEY: &str = "app_settings";
const TIMER_STATE_KEY: &str = "timer_state";
const TRAY_ID: &str = "pomodoro-tray";

include!("core/models.rs");
include!("data/storage.rs");
include!("core/timer.rs");
include!("features/remote.rs");
include!("features/analytics.rs");
include!("app/commands.rs");
include!("app/bootstrap.rs");

#[cfg(test)]
mod tests;
