//! tafs for Linux: the portable part, ported from the iPad app (kiff/tafs `ios/Tafs`).
//! Words, KIFF's sign-in, the kept session, the device key, the theme and the sidebar.
//! No UI: the GTK app places what this says.

pub mod access;
pub mod account;
pub mod account_time;
pub mod activity;
pub mod calendar;
pub mod card;
pub mod cards;
pub mod connect;
pub mod devices;
pub mod halt;
pub mod held;
pub mod home;
pub mod keys;
pub mod language;
pub mod money;
pub mod needs_you;
pub mod overview;
pub mod pairing;
pub mod people;
pub mod recipes;
pub mod record;
pub mod record_search;
pub mod sample;
pub mod session;
pub mod sidebar;
pub mod signin;
pub mod taf_page;
pub mod theme;
pub mod unproven;
pub mod usage;
pub mod waits;
pub mod words;
pub mod zones;
