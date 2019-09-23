use rand::prelude::*;
use rbc::game::{Square, Color, BoardState, PieceKind};
use rbc::ai_interface::Ai;
use rbc::infoset::Infoset;
use rbc::distr;

fn main() {
    dbg!(std::mem::size_of::<BoardState>());
    // let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    // log::set_max_level(log::LevelFilter::Info);

    let timer = std::time::Instant::now();

    let mut rng = StdRng::seed_from_u64(424242);

    let ai1 = rbc::greedy::GreedyAi { experiment: false };
    let ai2 = rbc::greedy::GreedyAi { experiment: false };

    let mut player1 = ai1.make_player(Color::White, 424242);
    let mut player2 = ai2.make_player(Color::Black, 424242);
    let mut infoset1 = Infoset::new(Color::White);
    let mut infoset2 = Infoset::new(Color::Black);

    let mut board = BoardState::initial();
    let mut move_number = 0;
    let mut last_capture_square = None;

    let mut html = std::io::sink();

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

        let player = match board.side_to_play() {
            Color::White => &mut player1,
            Color::Black => &mut player2,
        };
        let infoset = match board.side_to_play() {
            Color::White => &mut infoset1,
            Color::Black => &mut infoset2,
        };

        if move_number > 0 {
            infoset.opponent_move(last_capture_square);
            player.handle_opponent_move(last_capture_square, infoset, &mut html);
        }
        let sense_distr = player.choose_sense(infoset, &mut html);
        let sense = *distr::draw(&sense_distr, &mut rng);
        let sense_result = board.sense(sense);
        infoset.sense(sense, &sense_result);
        player.handle_sense(sense, &sense_result, infoset, &mut html);
        let requested_move_distr = player.choose_move(infoset, &mut html);
        let requested_move = *distr::draw(&requested_move_distr, &mut rng);
        if let Some(rm) = &requested_move {
            let mut fog_state = board.clone();
            fog_state.fog_of_war(board.side_to_play());
            assert!(fog_state.all_sensible_requested_moves().contains(rm));
        }
        let taken_move = requested_move.and_then(|m| board.requested_to_taken(m));
        last_capture_square = board.make_move(taken_move);
        infoset.my_move(requested_move, taken_move, last_capture_square);
        player.handle_move(requested_move, taken_move, last_capture_square, infoset, &mut html);

        move_number += 1;
    }
    println!("{:#?}", board.render());
    println!("white summary:\n{}", player1.get_summary());
    println!("black summary:\n{}", player2.get_summary());

    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
}
