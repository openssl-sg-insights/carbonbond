use carbonbond::{
    api::api_impl,
    api::api_trait::RootQueryRouter,
    api::query,
    config,
    custom_error::{Error, ErrorCode, Fallible},
    Ctx,
};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper_staticfile::Static;
use redis;
use std::path::PathBuf;

static mut INDEX: String = String::new();
fn index() -> &'static str {
    unsafe { &INDEX }
}

async fn on_request(
    req: Request<Body>,
    static_files: Static,
) -> Result<Response<Body>, hyper::Error> {
    let resp = on_request_inner(req, static_files).await;
    match resp {
        Ok(body) => Ok(body),
        Err(err) => {
            let err_msg = serde_json::to_string(&err).unwrap_or(String::default());
            let mut err_body = Response::new(Body::from(err_msg));
            *err_body.status_mut() = StatusCode::BAD_REQUEST;
            Ok(err_body)
        }
    }
}

async fn on_request_inner(req: Request<Body>, static_files: Static) -> Fallible<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/api") => {
            let (parts, body) = req.into_parts();

            let body = hyper::body::to_bytes(body).await?;
            log::trace!("原始請求： {:#?}", body);

            let query: query::RootQuery = serde_json::from_slice(&body.to_vec())
                .map_err(|e| Error::new_logic(ErrorCode::ParsingJson, &e))?;
            log::info!("請求： {:#?}", query);

            let root: api_impl::RootQueryRouter = Default::default();

            let mut context = Ctx {
                headers: parts.headers,
                resp: Response::new(String::new()),
            };

            let ret = root.handle(&mut context, query).await?;
            context.resp.body_mut().push_str(&ret);
            Ok(context.resp.map(|s| Body::from(s)))
        }
        (&Method::GET, _) => {
            if req.uri().path().starts_with("/app") {
                // html 檔案
                Ok(Response::new(Body::from(index())))
            } else {
                log::trace!("靜態資源： {}", req.uri().path());
                static_files.clone().serve(req).await.map_err(|e| e.into())
            }
        }
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化紀錄器
    env_logger::init();
    // 載入設定
    let args_config = clap::load_yaml!("args.yaml");
    let arg_matches = clap::App::from_yaml(args_config).get_matches();
    let config_file = arg_matches
        .value_of("config_file")
        .map(|p| PathBuf::from(p));
    config::initialize_config(&config_file);
    let conf = config::get_config();
    // TODO: 初始化資料庫連線池？
    log::info!("資料庫位置：{}", &conf.database.url);
    // 載入前端資源
    let static_files = Static::new("./frontend/static");
    // 載入首頁
    unsafe {
        let content =
            std::fs::read_to_string("./frontend/static/index.html").expect("讀取首頁失敗");
        INDEX = content;
    }
    // 打開伺服器
    let addr: std::net::SocketAddr =
        format!("{}:{}", &conf.server.address, &conf.server.port).parse()?;
    let service = make_service_fn(|_| {
        let static_files = static_files.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req| on_request(req, static_files.clone())))
        }
    });
    let server = Server::bind(&addr).serve(service);
    log::info!("Listening on http://{}", addr);
    server.await?;

    Ok(())
}
