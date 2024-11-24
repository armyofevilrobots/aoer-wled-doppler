use astra::{Body, Request, Response, ResponseBuilder, Server};
use http::method::Method;
use log::{self, info}; 
use ron;
use matchit::Match;
use std::{
    collections::HashMap,
    io::Read,
    sync::{Arc, Mutex},
};

use crate::Config;

type Router = matchit::Router<fn(&Arc<Mutex<State>>, Request) -> Response>;
type Params = HashMap<String, String>;

pub(crate) struct State {
    pub cfg: Config,
}

fn home(_state: &Arc<Mutex<State>>, _: Request) -> Response {
    Response::new(Body::new("Hello World via router!"))
}

fn config(state: &Arc<Mutex<State>>, mut req: Request) -> Response {
    info!("Req: {:?}", req);
    let cfg_path = {
        let cfg_tmp = state.lock().expect("Failed to lock state var.");
        cfg_tmp.cfg.config_path.clone()
    };
    let current_cfg = crate::config::load_config(cfg_path.clone());
    let req_method = req.method().clone();
    // let &mut in_body = req.body_mut();
    // let in_body: Config = serde_json::from_slice(in_body.collect()).unwrap();
    // let in_body: Result<Config, _> = serde_json::from_reader(req.body().reader());
    let in_body = req.body_mut();
    let mut body_string = String::new();
    in_body.reader().read_to_string(&mut body_string).unwrap();

    // Loading the config from this server
    match &req_method {
        &Method::GET => match current_cfg {
            Ok(cfg) => {
                let (status, out) = match serde_json::to_string_pretty(&cfg) {
                    Ok(jsonstr) => (200, jsonstr),
                    Err(_err) => (
                        500,
                        "{{\"err\":\"Failed to decode my own config file.\"}}".to_string(),
                    ),
                };
                ResponseBuilder::new()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Body::new(out))
                    .unwrap()
            }
            Err(err) => ResponseBuilder::new()
                .status(500)
                .body(Body::new(format!("Failed to load CFG file: {:?}", err)))
                .unwrap(),
        },

        // PUT receives a new config
        &Method::PUT => {
            info!("Got a PUT with data: {:?}", &body_string);
            match serde_json::from_str::<Config>(body_string.as_str()) {
                Ok(cfg_json) => match ron::ser::to_string_pretty(&cfg_json, ron::ser::PrettyConfig::default()) {
                    Ok(json_as_string) => {
                        let (status, outbody) = match std::fs::write(
                            &cfg_path.clone().expect("Invalid config path in server"),
                            &json_as_string.as_bytes(),
                        ) {
                            Ok(_) => (200, json_as_string.clone()),
                            Err(_) => (400, format!("{{\"Err\":\"Failed to save JSON.\"}}")),
                        };
                        ResponseBuilder::new()
                            .status(status)
                            .header("Content-Type", "application/json")
                            .body(Body::new(outbody))
                            .unwrap()
                    }
                    Err(_) => ResponseBuilder::new()
                        .status(400)
                        .header("Content-Type", "application/json")
                        .body(Body::new("{{\"Err\":\"Not Implemented\"}}"))
                        .unwrap(),
                },
                Err(_err) => ResponseBuilder::new()
                    .status(400)
                    .header("Content-Type", "application/json")
                    .body(Body::new("{{\"Err\":\"Invalid JSON data.\"}}"))
                    .unwrap(),
            }
        }

        // Any other method is invalid.
        _ => ResponseBuilder::new()
            .status(405)
            .header("Content-Type", "application/json")
            .body(Body::new(format!(
                "{{\"Err\":\"Invalid request type {}\"}}",
                &req_method
            )))
            .unwrap(),
    }
}

pub(crate) fn spawn(cfg: Config) {
    let state = Arc::new(Mutex::new(State { cfg }));

    let router = Arc::new({
        let mut router = Router::new();
        router.insert("/", home).unwrap();
        router.insert("/json/config", config).unwrap();
        // router.insert("/user/:id", get_user).unwrap();
        router
    });
    let bind_address = state
        .lock()
        .expect("Failed to lock state")
        .cfg
        .bind_address
        .clone()
        .unwrap();
    let router_state = state.clone();
    Server::bind(bind_address)
        .serve(move |req, _info| route(&router_state, router.clone(), req))
        .expect("serve failed");
}

fn route(state: &Arc<Mutex<State>>, router: Arc<Router>, mut req: Request) -> Response {
    // Try to find the handler for the requested path
    match router.at(req.uri().path()) {
        // If a handler is found, insert the route parameters into the request
        // extensions, and call it
        Ok(Match { value, params }) => {
            let params = params
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect::<Params>();
            req.extensions_mut().insert(params);
            (value)(&state, req)
        }
        // Otherwise return a 404
        Err(_) => ResponseBuilder::new()
            .status(404)
            .body(Body::empty())
            .unwrap(),
    }
}
