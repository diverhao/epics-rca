use super::log::init_log;
use crate::env::env::Env;
use crate::udp::udp::UDP;
use ::log::LevelFilter;
use ::log::info;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

pub static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

pub struct Runtime {
    context: Option<Context>,
}

impl Runtime {
    fn new() -> Self {
        Self { context: None }
    }

    fn set_context(&mut self, context: Context) {
        self.context = Some(context);
    }

    pub fn context(&self) -> &Context {
        self.context.as_ref().expect("context has not been created")
    }
}

pub struct Context {
    pub env: Env,
    log_level: LevelFilter,
    pub udp: UDP,
}

impl Context {
    pub async fn create(user_env: Vec<(&str, &str)>, log_level: LevelFilter) {
        init_log(log_level);

        let mut runtime = Runtime::new();
        let context = Context {
            env: Env::new(user_env),
            log_level,
            udp: UDP::new().await,
        };

        info!(
            "This EPICS client runs with following settings: \n{}",
            context.env
        );

        runtime.set_context(context);

        if RUNTIME.set(Mutex::new(runtime)).is_err() {
            panic!("runtime has already been created");
        }
    }
}

pub async fn create_context(user_env: Vec<(&str, &str)>, log_level: LevelFilter) {
    Context::create(user_env, log_level).await
}

pub fn get_runtime() -> MutexGuard<'static, Runtime> {
    RUNTIME
        .get()
        .expect("runtime has not been created")
        .lock()
        .expect("runtime mutex is poisoned")
}
