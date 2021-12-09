#![allow(dead_code)]
#![allow(unused_imports)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, deprecated, unused_variables))]

extern crate dirs;
extern crate thrussh;
extern crate thrussh_keys;
extern crate futures;
extern crate tokio;
extern crate env_logger;

use std::error;
use std::sync::Arc;
use thrussh::*;
use thrussh_keys::*;
//use crate::futures::TryFutureExt;
use futures::channel::oneshot::{self, Sender};

//#[macro_use] extern crate log;
use std::fmt;
use futures::TryFutureExt;
use log::debug;

pub fn run() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            start().await;
        })
}


#[derive(Debug)]
pub enum ExecErrorKind {
    HomeDirError
}

#[derive(Debug)]
pub struct ExecError {
    message: String,
}


impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.message)
    }
}

impl error::Error for ExecError {}


type ExecResult<T> = std::result::Result<T, Box<dyn error::Error>>;

struct Client {
    tx: Option<Sender<String>>,
}

impl client::Handler for Client {
    type Error = anyhow::Error;  //thrussh::Error
    type FutureBool = futures::future::Ready<Result<(Self, bool), Self::Error>>;
    type FutureUnit = futures::future::Ready<Result<(Self, client::Session), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        debug!("finish_bool");
        futures::future::ready(Ok((self, b)))
    }

    fn finished(self, sess: thrussh::client::Session) -> Self::FutureUnit {
        debug!("finished");
        futures::future::ready(Ok((self, sess)))
    }

    fn auth_banner(self, banner: &str, session: client::Session) -> Self::FutureUnit {
        debug!("auth_banner: {:?}", banner);
        self.finished(session)
    }

    fn check_server_key(self, server_public_key: &key::PublicKey) -> Self::FutureBool {
        debug!("check_server_key: {:?}", server_public_key);
        self.finished_bool(true)
    }

    fn channel_open_failure(self, _channel: ChannelId, reason: ChannelOpenFailure, _description: &str, _language: &str, session: client::Session) -> Self::FutureUnit {
        debug!("channel_open_failure {:?}", reason);
        self.finished(session)
    }

    fn data(mut self, channel: ChannelId, data: &[u8], session: client::Session) -> Self::FutureUnit {
        debug!("data on channel {:?}", channel);
        if let Some(tx) = self.tx.take() {
            match std::str::from_utf8(data) {
                Ok(s) => { Some(tx.send(s.to_string())); }
                Err(_) => drop(tx),
            }
        }
        self.finished(session)
    }

    fn extended_data(self, channel: ChannelId, _ext: u32, data: &[u8], session: client::Session) -> Self::FutureUnit {
        debug!("extended_data on channel {:?}: {:?}", channel, std::str::from_utf8(data));
        self.finished(session)
    }
}


/*async fn channel_read_loop(mut channel: thrussh::client::Channel) {
    while let Some(msg) = channel.wait().await {
        if let thrussh::ChannelMsg::Eof {..} = msg {
            break;
        }
    }
}*/

async fn exec_uname() -> ExecResult<String> {
    let config = thrussh::client::Config::default();
    let config = Arc::new(config);

    let (tx, rx) = oneshot::channel::<String>();
    let client_handler = Client { tx: Some(tx) };

   /* let mut privkey_path = dirs::home_dir().ok_or(ExecError { message: "No home dir".to_string() })?;
    privkey_path.push(".ssh");
    privkey_path.push("id_rsa");
    debug!("Private key found: {:?}", privkey_path);
    let key = thrussh_keys::load_secret_key(privkey_path, None)?;
    let key = Arc::new(key);*/

    let mut session = thrussh::client::connect(config, "192.168.133.13:22", client_handler).await?;
    //let b = session.authenticate_publickey("root", key).await?;
    let b = session.authenticate_password("isaque.neves", "Ins257257").await?;
    debug!("Session auth result: {}", b);

    let mut channel = session.channel_open_session().await?;
    channel.exec(true, "uname -a").await?;

    //channel_read_loop(channel);

    let f = rx.into_future().await?;
    Ok(f)
}

async fn start() {
    env_logger::init();
    match exec_uname().await {
        Ok(s) => println!("uname result: {}", s),
        Err(e) => println!("error: {}", e)
    };
}