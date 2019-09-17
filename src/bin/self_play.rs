use rbc::game::{STARTING_FEN, Square, Color, BoardState, PieceKind};
use rbc::ai_interface::Ai;

fn main() {
    dbg!(std::mem::size_of::<BoardState>());
    // let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    // log::set_max_level(log::LevelFilter::Info);

    let timer = std::time::Instant::now();

    let ai1 = rbc::greedy::GreedyAi;
    let ai2 = rbc::greedy::GreedyAi;

    let mut player1 = ai1.make_player(Color::White, 424242);
    let mut player2 = ai2.make_player(Color::Black, 424242);

    let mut board: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
    let mut move_number = 0;
    let mut last_capture_square = None;

    loop {
        dbg!(move_number);
        let mut white_king = false;
        let mut black_king = false;
        for sq in 0..64 {
            if let Some(p) = board.get_piece(Square(sq)) {
                if p.kind == PieceKind::King {
                    match p.color {
                        Color::White => white_king = true,
                        Color::Black => black_king = true,
                    }
                }
            }
        }
        assert!(white_king || black_king);
        if !white_king {
            println!("black won");
            break;
        }
        if !black_king {
            println!("white won");
            break;
        }

        let player = match board.side_to_play {
            Color::White => &mut player1,
            Color::Black => &mut player2,
        };

        if move_number > 0 {
            player.handle_opponent_move(last_capture_square);
        }
        let sense = player.choose_sense();
        player.handle_sense(sense, &board.sense(sense));
        let requested_move = player.choose_move();
        if let Some(rm) = &requested_move {
            let mut fog_state = board.clone();
            fog_state.fog_of_war(board.side_to_play);
            assert!(fog_state.all_sensible_requested_moves().contains(rm));
        }
        let taken_move = requested_move.map_or(None, |m| board.requested_to_taken(m));
        last_capture_square = board.make_move(taken_move);
        player.handle_move(requested_move, taken_move, last_capture_square);

        move_number += 1;
    }
    println!("{:#?}", board.render());
    println!("white summary:\n{}", player1.get_summary());
    println!("black summary:\n{}", player2.get_summary());

    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
}
