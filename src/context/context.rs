use super::log::init_log;
use crate::env::env::Env;
use crate::udp::udp::UDP;
use ::log::LevelFilter;
use ::log::error;
use ::log::info;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

/**
 * Global singleton Context storage.
 *
 * `OnceLock` ensures the Context is initialized at most once, and `Mutex`
 * provides synchronized mutable access to the Context after it has been
 * created.
 */
pub static CONTEXT: OnceLock<Mutex<Context>> = OnceLock::new();

pub struct Context {
    env: Env,
    log_level: LevelFilter,
    udp: UDP,
    // placeholder
    pub tcp: Option<i32>,
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
        let udp = UDP::new(&env).await;

        let context = Context {
            env,
            log_level,
            udp,
            tcp: None,
        };

        info!(
            "This EPICS client runs with following settings: \n{}",
            context.env
        );

        if CONTEXT.set(Mutex::new(context)).is_err() {
            panic!("Failed to create Context. Quit epics-rca.");
        }
    }

    // -------------- getters and setters ----------------

    pub fn set_log_level(&mut self, level: LevelFilter) {
        self.log_level = level;
        ::log::set_max_level(level);
    }

    pub fn log_level(self: &Self) -> &LevelFilter {
        &self.log_level
    }

    pub fn udp(self: &Self) -> &UDP {
        &self.udp
    }

    pub fn udp_mut(self: &mut Self) -> &mut UDP {
        &mut self.udp
    }

    /**
     * Env is not mutable
     */
    pub fn env(self: &Self) -> &Env {
        &self.env
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
pub fn get_context() -> MutexGuard<'static, Context> {
    CONTEXT
        .get()
        .expect("context has not been created")
        .lock()
        .expect("context mutex is poisoned")
}
