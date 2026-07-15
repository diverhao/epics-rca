use super::log::init_log;
use crate::env::env::Env;
use crate::udp::udp::UDP;
use ::log::LevelFilter;
use ::log::error;
use ::log::info;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::ca_channel::ca_channel::CaChannel;
use crate::ca_channel::ca_channels::CaChannels;
use crate::tcp::tcp::TCPs;

/**
 * Global singleton Context storage.
 *
 * `OnceLock` ensures the Context is initialized at most once, and `Mutex`
 * provides synchronized mutable access to the Context after it has been
 * created.
 */
pub static CONTEXT: OnceLock<Arc<Context>> = OnceLock::new();

pub struct Context {
    env: Env,
    udp: Arc<UDP>,
    ca_tcps: Arc<TCPs>,
    ca_channels: Arc<CaChannels>,
}
/**
 * Access point for the client runtime state. Can be accessed by calling
 * get_context().
 *
 * `Context` is stored in the global `CONTEXT` value, which is wrapped in
 * `OnceLock` to enforce singleton initialization and `Mutex` to allow safe
 * mutable access after creation.
 *
 * Call `Context::create` once during startup. If the global context has
 * already been created, the call is skipped and the existing context remains
 * unchanged.
 *
 * If the CONTEXT is not initialized, the program will panic when get_context()
 * is called.
 *
 */
impl Context {
    // factory method creating Context struct wrapped in OnceLock
    pub async fn create(user_env: Vec<(&str, &str)>, log_level: LevelFilter) {
        if CONTEXT.get().is_some() {
            error!("Context has already been created. Skip.");
            return;
        }

        init_log(log_level);
        let env = Env::new(user_env);
        let udp: Arc<UDP> = Arc::new(UDP::new(&env).await);
        Arc::clone(&udp).start_to_listen();

        let ca_channels = CaChannels::new();
        let ca_tcps = TCPs::new();

        let context = Context {
            env,
            udp: Arc::clone(&udp),
            ca_tcps: Arc::new(ca_tcps),
            ca_channels: Arc::new(ca_channels),
        };

        info!(
            "This EPICS client runs with following settings: \n{}",
            context.env
        );

        if CONTEXT.set(Arc::new(context)).is_err() {
            panic!("Failed to create Context. Quit epics-rca.");
        }
    }

    // -------------- channel ----------------------------

    pub fn create_ca_channel(self: &Self, name: &str) -> Arc<CaChannel> {
        let channels = self.ca_channels();
        channels.create_channel(name)
    }

    pub fn create_ca_channels(self: &Self, names: Vec<String>) {
        let channels = self.ca_channels();
        channels.create_channels(names)
    }

    pub fn start_search_ca(self: &Self) {
        let channels = self.ca_channels();
        channels.start_search();
    }

    // -------------- getters and setters ----------------

    pub fn set_log_level(level: LevelFilter) {
        ::log::set_max_level(level);
    }

    pub fn log_level() -> LevelFilter {
        ::log::max_level()
    }

    pub fn udp(self: &Self) -> Arc<UDP> {
        Arc::clone(&self.udp)
    }

    pub fn env(self: &Self) -> &Env {
        &self.env
    }

    pub fn ca_channels(self: &Self) -> Arc<CaChannels> {
        Arc::clone(&self.ca_channels)
    }

    pub fn tcps(self: &Self) -> Arc<TCPs> {
        Arc::clone(&self.ca_tcps)
    }
}

/**
 * Create the global singleton Context.
 *
 * Initializes logging, builds the environment from `user_env`, creates the UDP
 * sockets, and stores the Context in the global `CONTEXT` value. If the
 * Context has already been created, this function returns without changing the
 * existing Context.
 *
 * `user_env` contains user-provided environment overrides. `log_level`
 * controls the logger initialized during Context creation.
 */
pub async fn create_context(user_env: Vec<(&str, &str)>, log_level: LevelFilter) {
    Context::create(user_env, log_level).await;
}

/**
 * Get the global singleton Context.
 *
 * This function locks the global Context mutex and returns a `MutexGuard`.
 * While the guard exists, other callers must wait to lock the Context. The
 * Context is unlocked automatically when the guard is dropped.
 *
 * Panics if the Context has not been created or if the mutex is poisoned.
 */
pub fn get_context() -> Arc<Context> {
    Arc::clone(CONTEXT.get().expect("context has not been created"))
}
