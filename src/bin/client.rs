use log::info;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

const SERVER_URL: &str = "https://rbc.jhuapl.edu";

// let auth = base64::encode(&format!("{}:{}", "genetic", "***REMOVED***"));
// let auth = format!("Basic {}", auth);
const AUTH: &str = "Basic ***REMOVED***";

type MyError = Box<dyn std::error::Error>;
type MyResult<T> = Result<T, MyError>;

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

fn list_users() -> MyResult<Vec<String>> {
    Ok(make_get_request::<UsersResponse>("/api/users/")?
       .usernames)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct UsersMeResponse {
    id: i32,
    username: String,
    max_games: i32,
}

fn announce_myself() -> MyResult<UsersMeResponse> {
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

fn list_invitations() -> MyResult<Vec<i32>> {
    Ok(make_get_request::<ListInvitationsResponse>("/api/invitations/")?
       .invitations)
}

fn accept_invitation(inv_id: i32) -> MyResult<i32> {
    Ok(make_post_request::<_, AcceptInvitationResponse>(&format!("/api/invitations/{}", inv_id), &())?
       .game_id)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct GameStatusResponse {
    is_my_turn: bool,
    is_over: bool,
}

fn game_status(game_id: i32) -> MyResult<GameStatusResponse> {
    make_get_request(&format!("/api/games/{}/game_status", game_id))
}

#[derive(Debug)]
#[derive(Deserialize)]
struct GameColorResponse {
    color: bool,
}

fn game_color(game_id: i32) -> MyResult<bool> {
    make_get_request::<GameColorResponse>(&format!("/api/games/{}/color", game_id))
    .map(|r| r.color)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct WinnerColorResponse {
    winner_color: bool,
}

fn winner_color(game_id: i32) -> MyResult<bool> {
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

fn win_reason(game_id: i32) -> MyResult<WinReason> {
    let wr: WinReasonResponse = make_get_request(&format!("/api/games/{}/win_reason", game_id))?;
    Ok(wr.win_reason)
}

#[derive(Debug)]
#[derive(Deserialize)]
struct SecondsLeftResponse {
    seconds_left: f32,
}

fn seconds_left(game_id: i32) -> MyResult<f32> {
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
struct SenseResponse {
    sense_result: Vec<(i32, Option<Piece>)>,
}

fn sense(game_id: i32, square: i32) -> MyResult<SenseResponse> {
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
struct MoveResponse {
    move_result: (Option<Move>, Option<Move>, Option<i32>),  // (requested, taken, capture square)
}

fn make_move(game_id: i32, m: Move) -> MyResult<MoveResponse> {
    make_post_request(&format!("/api/games/{}/move", game_id), &MoveRequest { requested_move: m })
}

#[derive(Debug)]
#[derive(Deserialize)]
struct EndMoveResponse {}

fn end_turn(game_id: i32) -> MyResult<()> {
    make_post_request::<_, EndMoveResponse>(&format!("/api/games/{}/end_turn", game_id), &())?;
    Ok(())
}

#[derive(Debug)]
#[derive(Deserialize)]
struct OpponentMoveResultsResponse {
    opponent_move_results: Option<i32>,
}

fn opponent_move_results(game_id: i32) -> MyResult<Option<i32>> {
    let addr = format!("/api/games/{}/opponent_move_results", game_id);
    Ok(make_get_request::<OpponentMoveResultsResponse>(&addr)?
       .opponent_move_results)
}

fn main() {
    env_logger::init();

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        info!("Ctrl-C, entering lame duck mode");
        r.store(false, Ordering::SeqCst);
    }).unwrap();

    list_users().expect("TODO");
    let me = announce_myself().expect("TODO");

    let mut game_ids: Vec<i32> = Vec::new();
    loop {
        info!("active games: {:?}", game_ids);

        if game_ids.is_empty() && !running.load(Ordering::SeqCst) {
            info!("done");
            break;
        }

        game_ids.retain(|&game_id| {
            let gs = game_status(game_id).expect("TODO");
            if gs.is_over {
                let my_color = game_color(game_id).expect("TODO");
                let winner = winner_color(game_id).expect("TODO");
                let win_reason = win_reason(game_id).expect("TODO");
                if my_color == winner {
                    info!("I won game {} ({})", game_id, win_reason.0);
                } else {
                    info!("I lost game {} ({})", game_id, win_reason.0);
                }
                false
            } else {
                if gs.is_my_turn {
                    seconds_left(game_id).expect("TODO");
                    opponent_move_results(game_id).expect("TODO");
                    sense(game_id, 0).expect("TODO");
                    make_move(game_id, Move("d2d4".to_owned())).expect("TODO");
                    end_turn(game_id).expect("TODO");
                }
                true
            }
        });

        if running.load(Ordering::SeqCst) {
            for inv_id in list_invitations().expect("TODO") {
                if game_ids.len() < me.max_games as usize {
                    game_ids.push(accept_invitation(inv_id).expect("TODO"));
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
