use rbc::cfr::{NodeInfo, Game, Encoding, Cfr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Coin {
    Heads,
    Tails,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CoinTossAction {
    Toss(Coin),
    Sell, Play,
    Guess(Coin),
    Forfeit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CoinTossInfoset {
    SellOrPlay(Coin),
    Guess,
}

#[derive(Debug)]
struct CoinTossGame;

impl Game for CoinTossGame {
    type Action = CoinTossAction;
    type Infoset = CoinTossInfoset;

    fn node_info(&self, h: &[Self::Action]) -> NodeInfo<Self::Action, Self::Infoset> {
        match h {
            [] => NodeInfo::Chance(vec![
                (0.5, CoinTossAction::Toss(Coin::Heads)),
                (0.5, CoinTossAction::Toss(Coin::Tails)),
            ]),

            [CoinTossAction::Toss(coin)] => NodeInfo::Choice {
                player: 0,
                infoset: CoinTossInfoset::SellOrPlay(*coin),
                actions: vec![CoinTossAction::Sell, CoinTossAction::Play],
            },

            [CoinTossAction::Toss(Coin::Heads),
             CoinTossAction::Sell,
            ] => NodeInfo::Terminal(0.5),

            [CoinTossAction::Toss(Coin::Tails),
             CoinTossAction::Sell,
            ] => NodeInfo::Terminal(-0.5),

            [CoinTossAction::Toss(_),
             CoinTossAction::Play,
            ] => NodeInfo::Choice {
                player: 1,
                infoset: CoinTossInfoset::Guess,
                actions: vec![
                    CoinTossAction::Guess(Coin::Heads),
                    CoinTossAction::Guess(Coin::Tails),
                    CoinTossAction::Forfeit,
                ],
            },

            [CoinTossAction::Toss(actual),
             CoinTossAction::Play,
             CoinTossAction::Guess(guess),
            ] => NodeInfo::Terminal(
                if actual == guess { -1.0 } else { 1.0 }
            ),

            [CoinTossAction::Toss(_),
             CoinTossAction::Play,
             CoinTossAction::Forfeit,
            ] => NodeInfo::Terminal(1.0),

            _ => unreachable!("{:?}", h),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Rps {
    Rock,
    Paper,
    Scissors,
}

#[derive(Debug)]
struct RpsGame;

impl Game for RpsGame {
    type Action = Rps;
    type Infoset = u8;

    fn node_info(&self, h: &[Self::Action]) -> NodeInfo<Self::Action, Self::Infoset> {
        match h {
            [] => NodeInfo::Choice {
                player: 0,
                infoset: 0,
                actions: vec![Rps::Rock, Rps::Paper, Rps::Scissors],
            },
            [_] => NodeInfo::Choice {
                player: 1,
                infoset: 1,
                actions: vec![Rps::Rock, Rps::Paper, Rps::Scissors],
            },
            [Rps::Rock,     Rps::Rock    ] => NodeInfo::Terminal( 0.0),
            [Rps::Rock,     Rps::Paper   ] => NodeInfo::Terminal(-1.0),
            [Rps::Rock,     Rps::Scissors] => NodeInfo::Terminal( 1.0),
            [Rps::Paper,    Rps::Rock    ] => NodeInfo::Terminal( 1.0),
            [Rps::Paper,    Rps::Paper   ] => NodeInfo::Terminal( 0.0),
            [Rps::Paper,    Rps::Scissors] => NodeInfo::Terminal(-1.0),
            [Rps::Scissors, Rps::Rock    ] => NodeInfo::Terminal(-1.0),
            [Rps::Scissors, Rps::Paper   ] => NodeInfo::Terminal( 2.0),
            [Rps::Scissors, Rps::Scissors] => NodeInfo::Terminal( 0.0),
            _ => unreachable!(),
        }
    }
}

fn main() {
    // let enc = Encoding::new(&CoinTossGame);
    let enc = Encoding::new(&RpsGame);
    dbg!(&enc);
    let mut cfr = Cfr::new(&enc);
    dbg!(&cfr);
    for _ in 0..1000 {
        cfr.step(&enc);
    }
    dbg!(&cfr);
    dbg!(cfr.get_strategy(&enc));
}
