use log::info;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

const SERVER_URL: &str = "https://rbc.jhuapl.edu";

// let auth = base64::encode(&format!("{}:{}", "genetic", "***REMOVED***"));
// let auth = format!("Basic {}", auth);
const AUTH: &str = "Basic ***REMOVED***";

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn make_get_request<Response: DeserializeOwned>(addr: &str) -> MyResult<Response> {
    info!("GET {}", addr);
    let req = minreq::get(format!("{}{}", SERVER_URL, addr))
        .with_header("Authorization", AUTH);
    let resp = req.send()?;
    info!("got {} {}", resp.status_code, resp.body.trim_end());
    if resp.status_code != 200 {
        Err(format!("{}", resp.status_code))?
    }
    Ok(serde_json::from_str(&resp.body)?)
}

fn make_post_request<Request: Serialize, Response: DeserializeOwned>(
    addr: &str, req: &Request) -> MyResult<Response>
{
    let payload = serde_json::to_string(req).expect("TODO");
    info!("POST {}, req: {}", addr, payload);
    let req = minreq::post(format!("{}{}", SERVER_URL, addr))
        .with_header("Authorization", AUTH)
        .with_body(payload);
    let resp = req.send()?;
    info!("got  {} {}", resp.status_code, resp.body.trim_end());
    if resp.status_code != 200 {
        Err(format!("{}", resp.status_code))?
    }
    Ok(serde_json::from_str(&resp.body)?)
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
struct TypeValue {
    #[serde(rename = "type")]
    tp: String,
    value: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
struct UsersResponse {
    usernames: Vec<String>,
}

pub fn list_users() -> MyResult<Vec<String>> {
    Ok(make_get_request::<UsersResponse>("/api/users/")?
       .usernames)
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct UsersMeResponse {
    id: i32,
    username: String,
    pub max_games: i32,
}

pub fn announce_myself() -> MyResult<UsersMeResponse> {
    make_post_request::<_, UsersMeResponse>("/api/users/me", &())
}

#[derive(Serialize)]
struct UsersMeMaxGamesRequest {
    max_games: i32,
}

#[derive(Debug)]
#[derive(Deserialize)]
struct ListInvitationsResponse {
    invitations: Vec<i32>,
}

#[derive(Debug)]
#[derive(Deserialize)]
struct AcceptInvitationResponse {
    game_id: i32,
}

pub fn list_invitations() -> MyResult<Vec<i32>> {
    Ok(make_get_request::<ListInvitationsResponse>("/api/invitations/")?
       .invitations)
}

pub fn accept_invitation(inv_id: i32) -> MyResult<i32> {
    Ok(make_post_request::<_, AcceptInvitationResponse>(&format!("/api/invitations/{}", inv_id), &())?
       .game_id)
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct GameStatusResponse {
    pub is_my_turn: bool,
    pub is_over: bool,
}

pub fn game_status(game_id: i32) -> MyResult<GameStatusResponse> {
    make_get_request(&format!("/api/games/{}/game_status", game_id))
}

#[derive(Debug)]
#[derive(Deserialize)]
struct GameColorResponse {
    color: bool,
}

pub fn game_color(game_id: i32) -> MyResult<bool> {
    make_get_request::<GameColorResponse>(&format!("/api/games/{}/color", game_id))
    .map(|r| r.color)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct WinnerColorResponse {
    winner_color: bool,
}

pub fn winner_color(game_id: i32) -> MyResult<bool> {
    Ok(make_get_request::<WinnerColorResponse>(&format!("/api/games/{}/winner_color", game_id))?
       .winner_color)
}

#[derive(Debug)]
#[derive(Deserialize)]
#[serde(from = "TypeValue")]
struct WinReason(String);

impl From<TypeValue> for WinReason {
    fn from(tv: TypeValue) -> WinReason {
        assert_eq!(tv.tp, "WinReason");
        WinReason(tv.value)
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
struct WinReasonResponse {
    win_reason: WinReason,
}

pub fn win_reason(game_id: i32) -> MyResult<String> {
    let wr: WinReasonResponse = make_get_request(&format!("/api/games/{}/win_reason", game_id))?;
    Ok(wr.win_reason.0)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct SecondsLeftResponse {
    seconds_left: f32,
}

pub fn seconds_left(game_id: i32) -> MyResult<f32> {
    Ok(make_get_request::<SecondsLeftResponse>(&format!("/api/games/{}/seconds_left", game_id))?
       .seconds_left)
}

#[derive(Serialize)]
struct SenseRequest {
    square: i32,
}

#[derive(Debug)]
#[derive(Deserialize)]
#[serde(from = "TypeValue")]
struct Piece(String);

impl From<TypeValue> for Piece {
    fn from(tv: TypeValue) -> Piece {
        assert_eq!(tv.tp, "Piece");
        Piece(tv.value)
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct SenseResponse {
    sense_result: Vec<(i32, Option<Piece>)>,
}

pub fn sense(game_id: i32, square: i32) -> MyResult<SenseResponse> {
    make_post_request(&format!("/api/games/{}/sense", game_id), &SenseRequest { square })
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[serde(into = "TypeValue")]
#[serde(from = "TypeValue")]
struct Move(String);

impl From<TypeValue> for Move {
    fn from(tv: TypeValue) -> Move {
        assert_eq!(tv.tp, "Move");
        Move(tv.value)
    }
}

impl From<Move> for TypeValue {
    fn from(m: Move) -> TypeValue {
        TypeValue {
            tp: "Move".to_owned(),
            value: m.0,
        }
    }
}

#[derive(Serialize)]
struct MoveRequest {
    requested_move: Move,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct MoveResponse {
    move_result: (Option<Move>, Option<Move>, Option<i32>),  // (requested, taken, capture square)
}

pub fn make_move(game_id: i32, m: String) -> MyResult<MoveResponse> {
    make_post_request(&format!("/api/games/{}/move", game_id), &MoveRequest { requested_move: Move(m) })
}

#[derive(Debug)]
#[derive(Deserialize)]
struct EndMoveResponse {}

pub fn end_turn(game_id: i32) -> MyResult<()> {
    make_post_request::<_, EndMoveResponse>(&format!("/api/games/{}/end_turn", game_id), &())?;
    Ok(())
}

#[derive(Debug)]
#[derive(Deserialize)]
struct OpponentMoveResultsResponse {
    opponent_move_results: Option<i32>,
}

pub fn opponent_move_results(game_id: i32) -> MyResult<Option<i32>> {
    let addr = format!("/api/games/{}/opponent_move_results", game_id);
    Ok(make_get_request::<OpponentMoveResultsResponse>(&addr)?
       .opponent_move_results)
}
