use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc, sync::mpsc::UnboundedSender, sync::mpsc::UnboundedReceiver};

const STORAGE_PATH: &str = "./recipes.json";

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

//type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

struct Recipe {
    id: u64,
    name: String,
    ingredients: String,
    instructions: String,
    public: bool
}

type Recipes = Vec<Recipe>;

enum ListMode {
    All,
    One(String)
}
struct ListRequest {
    mode: ListMode
}
struct ListResponse {
    mode: ListMode,
    data: Recipes,
    receiver: String
}
enum EventType {
    Response(ListResponse),
    Input(String)
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (response_sender, mut response_reciever): (
        UnboundedSender<Result<&str, Box<dyn std::error::Error + Send>>>,
        UnboundedReceiver<Result<&str, Box<dyn std::error::Error + Send>>>
    ) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&KEYS).expect("Can`t create auth keys");

    let transport = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = RecipeBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        mdns: TokioMdns::new().expect("can create mdns"),
        response_sender,
    };

    let mut swarm = SwarmBuilder::new(transport, behaviour, PEER_ID.clone())
    .executor(Box::new(|fut| {
        tokio::spawn(fut);
    }))
    .build();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");



    behaviour.floodsub.subscribe(TOPIC.clone());

    println!("Initialized");
}
