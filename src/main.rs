use axum::{routing::get, Router};
use axum_session::{
    SessionConfig, SessionLayer, SessionStore, SessionSurrealPool, SessionSurrealSession,
};
use surrealdb::engine::any::{connect, Any};
use surrealdb::engine::local::Mem;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use yew::prelude::*;
use yew::ServerRenderer;

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;

use futures::stream::{self, StreamExt};

use axum::body::StreamBody;
use clap::Parser;
use axum::response::IntoResponse;
use axum::http::{StatusCode, Uri};
use axum::extract::{Query, State};

use yew_router::history::{AnyHistory, History, MemoryHistory};
use yew_router::prelude::*;



#[function_component]
fn App() -> Html {
    html! {<div>{"Hello, World FROM HTML CMPONENT!"}</div>}
}

#[derive(Properties, PartialEq, Eq, Debug)]
pub struct ServerAppProps {
    pub url: AttrValue,
    pub queries: HashMap<String, String>,
}

#[function_component]
pub fn ServerApp(props: &ServerAppProps) -> Html {
    let history = AnyHistory::from(MemoryHistory::new());
    history
        .push_with_query(&*props.url, &props.queries)
        .unwrap();

    html!{}

}



pub async fn no_main() {
    let renderer = ServerRenderer::<App>::new();

    let rendered = renderer.render().await;

    // Prints: <div>Hello, World!</div>
    println!("{}", rendered);
}


/// A basic example
#[derive(Parser, Debug)]
struct Opt {
    /// the "dist" created by trunk directory to be served for hydration.
    #[clap(short, long)]
    dir: PathBuf,
}

pub async fn render(
    url: Uri,
    Query(queries): Query<HashMap<String, String>>,
    State((index_html_before, index_html_after)): State<(String, String)>,
) -> impl IntoResponse {
    let url = url.to_string();

    let renderer = yew::ServerRenderer::<ServerApp>::with_props(move || ServerAppProps {
        url: url.into(),
        queries,
    });

    StreamBody::new(
        stream::once(async move { index_html_before })
            .chain(renderer.render_stream())
            .chain(stream::once(async move { index_html_after }))
            .map(Result::<_, Infallible>::Ok),
    )
}


async fn root() -> &'static str {
    "Hello, World from AXUM!"
}

async fn counter(session: SessionSurrealSession<Any>) -> String {
    let mut count: usize = session.get("count").unwrap_or(0);
    count += 1;
    session.set("count", count);
    let sessions_count = session.count().await;
    // consider use better Option handling here instead of expect
    let new_count = session.get::<usize>("count").expect("error setting count");
    format!("We have set the counter to surreal: {new_count}, Sessions Count: {sessions_count}")
}

#[tokio::main]
async fn main() {

    // Create the Surreal connection.
    //let db = Surreal::new::<Mem>(()).await.unwrap();
    
    // Create the Surreal connection.
    let db = connect("ws://localhost:8000").await.unwrap();

    // sign in as our account.
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await
    .unwrap();

    // Set the database and namespace we will function within.
    db.use_ns("test").use_db("test").await.unwrap();

    // No need here to specify a table name because redis does not support tables
    let session_config = SessionConfig::default();

    let session_store =
        SessionStore::new(Some(SessionSurrealPool::new(db.clone())), session_config)
            .await
            .unwrap();

    // initiate the database tables
    session_store.initiate().await.unwrap();


    let opts = Opt::parse();
    let index_html_s = tokio::fs::read_to_string(opts.dir.join("index.html"))
        .await
        .expect("failed to read index.html");

    let (index_html_before, index_html_after) = index_html_s.split_once("<body>").unwrap();
    let mut index_html_before = index_html_before.to_owned();
    index_html_before.push_str("<body>");

    let index_html_after = index_html_after.to_owned();




    // build our application with a single route
      let app = Router::new()
        .route("/", get(root))
        // `POST /users` goes to `counter`
        .route("/counter", get(counter))
        // try to get APP from yew
        .route("/yew", get(no_main))
        // try APP with render HTML
       
        .route("/yewq", get(render)
               .with_state((index_html_before.clone(), index_html_after.clone()))
               //     .into_service()
               //    .map_err(|err| -> std::io::Error { match err {} }),)
        )
        .layer(SessionLayer::new(session_store)); // adding the crate plugin ( layer ) to the project

    // run it with hyper on localhost:3000
    println!("Conntection is on: http://localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();






}

