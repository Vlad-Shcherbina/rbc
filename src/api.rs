use std::collections::HashMap;
use log::info;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

use crate::game::{Color, Piece};

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
        return Err(format!("{}", resp.status_code).into());
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
        return Err(format!("{}", resp.status_code).into());
    }
    Ok(serde_json::from_str(&resp.body)?)
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct TypeValue {
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

#[allow(dead_code)]  // TODO
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

impl From<bool> for Color {
    fn from(b: bool) -> Color {
        if b { Color::White } else { Color::Black }
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
struct GameColorResponse {
    color: Color,
}

pub fn game_color(game_id: i32) -> MyResult<Color> {
    make_get_request::<GameColorResponse>(&format!("/api/games/{}/color", game_id))
    .map(|r| r.color)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct WinnerColorResponse {
    winner_color: Color,
}

pub fn winner_color(game_id: i32) -> MyResult<Color> {
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

impl From<TypeValue> for Piece {
    fn from(tv: TypeValue) -> Piece {
        assert_eq!(tv.tp, "Piece");
        assert_eq!(tv.value.len(), 1);
        let c = tv.value.chars().next().unwrap();
        Piece::from_char(c)
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

#[derive(Debug)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::type_complexity)]
pub struct RawGameHistory {
    #[serde(rename = "type")]
    tp: String,  // "GameHistory"
    white_name: String,
    black_name: String,
    winner_color: Color,
    win_reason: WinReason,
    senses: HashMap<String, Vec<Option<i32>>>,
    sense_results: HashMap<String, Vec<Vec<(i32, Option<Piece>)>>>,
    requested_moves: HashMap<String, Vec<Option<Move>>>,
    taken_moves: HashMap<String, Vec<Option<Move>>>,
    capture_squares: HashMap<String, Vec<Option<i32>>>,
    fens_before_move: HashMap<String, Vec<String>>,
    fens_after_move: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct MoveHistory {
    sense: Option<i32>,
    sense_result: Vec<(i32, Option<Piece>)>,
    requested_move: Option<String>,
    taken_move: Option<String>,
    capture_square: Option<i32>,
    fen_before: String,
    fen_after: String,
}

#[derive(Debug)]
pub struct GameHistory {
    white_name: String,
    black_name: String,
    winner_color: Color,
    win_reason: String,
    moves: Vec<MoveHistory>,
}

impl From<RawGameHistory> for GameHistory {
    fn from(h: RawGameHistory) -> GameHistory {
        let white_moves = h.senses["true"].len();
        assert_eq!(white_moves, h.sense_results["true"].len());
        assert_eq!(white_moves, h.requested_moves["true"].len());
        assert_eq!(white_moves, h.taken_moves["true"].len());
        assert_eq!(white_moves, h.capture_squares["true"].len());
        assert_eq!(white_moves, h.fens_before_move["true"].len());
        assert_eq!(white_moves, h.fens_after_move["true"].len());
        let black_moves = h.senses["false"].len();
        assert_eq!(black_moves, h.sense_results["false"].len());
        assert_eq!(black_moves, h.requested_moves["false"].len());
        assert_eq!(black_moves, h.taken_moves["false"].len());
        assert_eq!(black_moves, h.capture_squares["false"].len());
        assert_eq!(black_moves, h.fens_before_move["false"].len());
        assert_eq!(black_moves, h.fens_after_move["false"].len());
        let mut moves = Vec::new();
        for i in 0..white_moves + black_moves {
            let color = if i % 2 == 0 { "true" } else { "false" };
            moves.push(MoveHistory {
                sense: h.senses[color][i / 2],
                sense_result: h.sense_results[color][i / 2].clone(),
                requested_move: h.requested_moves[color][i / 2].clone().map(|m| m.0),
                taken_move: h.taken_moves[color][i / 2].clone().map(|m| m.0),
                capture_square: h.capture_squares[color][i / 2],
                fen_before: h.fens_before_move[color][i / 2].clone(),
                fen_after: h.fens_after_move[color][i / 2].clone(),
            });
        }
        GameHistory {
            white_name: h.white_name,
            black_name: h.black_name,
            winner_color: h.winner_color,
            win_reason: h.win_reason.0,
            moves
        }
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GameHistoryResponse {
    game_history: RawGameHistory,
}

pub fn game_history(game_id: i32) -> MyResult<GameHistory> {
    let addr = format!("/api/games/{}/game_history", game_id);
    make_get_request::<GameHistoryResponse>(&addr)
    .map(|r| r.game_history.into())
}
