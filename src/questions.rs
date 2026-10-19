//! The World Check question banks: one bank per world, read-the-code questions.
//!
//! Every question shows a short program (in the save format, see
//! `Program::deserialize`) and sometimes a small map, then asks what happens.
//! **The first choice is always the right answer**; the game shuffles the
//! choices before showing them, so explanations must never point at a choice
//! by its position ("the middle program", "the last one").
//!
//! Each question also says how to prove its answer (`proof`). `cargo test`
//! runs every program in the game's own simulator and fails if a marked
//! answer is wrong, so new questions can't quietly teach the wrong thing.
//!
//! Map symbols are the same as for levels (see `World::from_level`). Count
//! questions without a map run on a big empty field, starting in the middle
//! and facing up.
//!
//! Condition numbers in programs: 0 path ahead, 1 path to left, 2 path to
//! right, 3 stone ahead, 4 diamond ahead, 5 lava ahead, 6 water ahead,
//! 7 not at portal.

use crate::program::Action;

pub struct Question {
    /// Stable and unique: saves remember which questions a player last saw.
    pub id: &'static str,
    pub prompt: &'static str,
    /// The program to read ("" for none). Main and function are split by `/`.
    pub program: &'static str,
    /// A small map to show (empty for none).
    pub map: &'static [&'static str],
    /// Blocks in the miner's backpack at the start.
    pub blocks: u32,
    /// The first one is the right answer.
    pub choices: Choices,
    /// Bit's explanation, shown after answering.
    pub explain: &'static str,
    /// Only read by the tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub proof: Proof,
}

pub enum Choices {
    Text(&'static [&'static str]),
    /// Programs to pick from, drawn side by side.
    Programs(&'static [&'static str]),
    /// Squares on the map, labelled A, B, C.
    Spots(&'static [(i32, i32)]),
}

/// How the tests check the answer.
#[cfg_attr(not(test), allow(dead_code))]
pub enum Proof {
    /// Running `program` does these actions this many times in total; the
    /// right answer is that number.
    Count(&'static [Action]),
    /// Running `program` on `map` ends on the right spot.
    EndsAt,
    /// Running `program` on `map` ends like this.
    Ends(End),
    /// Of the choice programs, only the right one wins on `map`, in every
    /// version of it (maps with `?`-style squares change each time).
    OnlyWinner,
    /// Of the choice programs, only the right one ends exactly like
    /// `program` does (same square, facing, backpack, map and result).
    SameResult,
    /// The number of blocks left in the backpack at the end.
    Backpack,
}

/// How a program run ends.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum End {
    Wins,
    /// The program finished without winning.
    Unfinished,
    Bonk,
    Hazard,
    NothingToMine,
    CantMine,
    NoBlocks,
    CantPlace,
    /// Ran too long (e.g. a WHILE loop that never ends).
    Stuck,
}

const Q: Question = Question {
    id: "",
    prompt: "",
    program: "",
    map: &[],
    blocks: 0,
    choices: Choices::Text(&[]),
    explain: "",
    proof: Proof::EndsAt,
};

use Action::{Forward, Mine, Place, TurnLeft, TurnRight};
const TURNS: &[Action] = &[TurnLeft, TurnRight];
const ALL_ACTIONS: &[Action] = &[Forward, TurnLeft, TurnRight, Mine, Place];

const ROOM_7: &[&str] = &["TTTTTTT", "T.....T", "T.....T", "T.....T", "T.....T", "T^....T", "TTTTTTT"];

// ============================ World 1: Sequences ============================
const W1: &[Question] = &[
    Question {
        id: "w1-01",
        prompt: "Where does the miner end up?",
        program: "F F R F F",
        map: &["TTTTTTT", "T>....T", "T.....T", "T.....T", "TTTTTTT"],
        choices: Choices::Spots(&[(3, 3), (5, 1), (2, 3)]),
        explain: "Two FORWARDs move 2 squares right. TURN RIGHT makes the miner face down, then 2 more FORWARDs go down.",
        ..Q
    },
    Question {
        id: "w1-02",
        prompt: "Where does the miner end up?",
        program: "F F F L F F",
        map: &["TTTTTTT", "T.....T", "T.....T", "T>....T", "TTTTTTT"],
        choices: Choices::Spots(&[(4, 1), (2, 3), (5, 1)]),
        explain: "Three steps right, then TURN LEFT. The miner was facing right, so its left is UP. Then two steps up.",
        ..Q
    },
    Question {
        id: "w1-03",
        prompt: "What happens when this program runs?",
        program: "F F F F",
        map: &["TTTTTTT", "T>.#.PT", "TTTTTTT"],
        choices: Choices::Text(&["It bumps into the stone", "It reaches the portal", "It mines the stone"]),
        explain: "FORWARD can't go through stone. The program never says MINE, so the miner bonks into it.",
        proof: Proof::Ends(End::Bonk),
        ..Q
    },
    Question {
        id: "w1-04",
        prompt: "Which program gets the miner to the portal?",
        map: &["TTTTTTT", "T>.#.PT", "TTTTTTT"],
        choices: Choices::Programs(&["F M F F F", "F F M F F", "M F F F F"]),
        explain: "Walk one step, MINE the stone right in front, then three steps to the portal. Mining too early swings at thin air; walking too far bonks the stone.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w1-05",
        prompt: "How many times does the miner move forward?",
        program: "F F R F L F F",
        choices: Choices::Text(&["5", "7", "3"]),
        explain: "Count only the FORWARD blocks: there are 5. The turns change direction but don't move the miner.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w1-06",
        prompt: "How many times does the miner turn?",
        program: "F R F F L F R",
        choices: Choices::Text(&["3", "7", "4"]),
        explain: "There are 2 TURN RIGHTs and 1 TURN LEFT: 3 turns. The other 4 blocks are FORWARDs.",
        proof: Proof::Count(TURNS),
        ..Q
    },
    Question {
        id: "w1-07",
        prompt: "What happens when this program runs?",
        program: "F F F",
        map: &["TTTTTT", "T>.LPT", "TTTTTT"],
        choices: Choices::Text(&["It steps into the lava", "It walks over the lava safely", "It reaches the portal"]),
        explain: "The second FORWARD walks straight into lava. To cross lava you need to PLACE a block on it first.",
        proof: Proof::Ends(End::Hazard),
        ..Q
    },
    Question {
        id: "w1-08",
        prompt: "The backpack starts empty. Which program reaches the portal?",
        map: &["TTTTTTT", "T>#.LPT", "TTTTTTT"],
        choices: Choices::Programs(&["M F F P F F", "M F F F F", "F M F P F F"]),
        explain: "MINE the stone to get a block, walk up to the lava, PLACE the block to make a bridge, then cross to the portal.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w1-09",
        prompt: "The backpack starts empty. What happens?",
        program: "F P F F",
        map: &["TTTTTT", "T>.LPT", "TTTTTT"],
        choices: Choices::Text(&[
            "It can't place a block: the backpack is empty",
            "It builds a bridge and reaches the portal",
            "It steps into the lava",
        ]),
        explain: "PLACE BLOCK needs a block in the backpack. The miner never mined anything, so there's nothing to place.",
        proof: Proof::Ends(End::NoBlocks),
        ..Q
    },
    Question {
        id: "w1-10",
        prompt: "What happens when this program runs?",
        program: "M F F F",
        map: &["TTTTTT", "T>..PT", "TTTTTT"],
        choices: Choices::Text(&["The pickaxe swings at thin air", "It reaches the portal", "It digs a tunnel"]),
        explain: "MINE only works when there's stone or ore in front. Here the square ahead is empty, so the program stops with an error.",
        proof: Proof::Ends(End::NothingToMine),
        ..Q
    },
    Question {
        id: "w1-11",
        prompt: "Where does the miner end up?",
        program: "M F F R F",
        map: &["TTTTTT", "T>#..T", "T....T", "TTTTTT"],
        choices: Choices::Spots(&[(3, 2), (4, 1), (2, 2)]),
        explain: "MINE clears the stone without moving. Then two steps right, TURN RIGHT to face down, and one step down.",
        ..Q
    },
    Question {
        id: "w1-12",
        prompt: "Which program ends up exactly the same as this one?",
        program: "R R F",
        choices: Choices::Programs(&["L L F", "R F", "L F"]),
        explain: "Two right turns spin the miner all the way around. Two left turns do that too, so both programs walk backwards one step.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w1-13",
        prompt: "Which program reaches the portal?",
        map: &["TTTTTTT", "T>..TTT", "TTT.TTT", "TTT..PT", "TTTTTTT"],
        choices: Choices::Programs(&["F F R F F L F F", "F F L F F R F F", "F F R F F R F F"]),
        explain: "Facing right, the path goes DOWN, which is to the miner's right. At the bottom it bends right again, which is now to the miner's LEFT.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w1-14",
        prompt: "How many blocks are in the backpack when the program ends?",
        program: "M F F M F F F",
        map: &["TTTTTTTT", "T>#.#.PT", "TTTTTTTT"],
        choices: Choices::Text(&["2", "0", "3"]),
        explain: "Each MINE on a stone puts one block in the backpack. The miner mined 2 stones and never placed a block.",
        proof: Proof::Backpack,
        ..Q
    },
    Question {
        id: "w1-15",
        prompt: "Where is the miner after this one-block program?",
        program: "R",
        map: &["TTTTT", "T...T", "T.^.T", "T...T", "TTTTT"],
        choices: Choices::Spots(&[(2, 2), (3, 2), (3, 1)]),
        explain: "TURN RIGHT only changes which way the miner faces. It doesn't take a step, so the miner stays put.",
        ..Q
    },
];

// ============================== World 2: Loops ==============================
const W2: &[Question] = &[
    Question {
        id: "w2-01",
        prompt: "How many steps forward does the miner take?",
        program: "[R4 F ]",
        choices: Choices::Text(&["4", "1", "5"]),
        explain: "REPEAT 4 runs the blocks inside it 4 times. There's one FORWARD inside, so that's 4 steps.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w2-02",
        prompt: "How many times does the miner turn right?",
        program: "[R3 F R ]",
        choices: Choices::Text(&["3", "1", "6"]),
        explain: "Each time round the loop there's one TURN RIGHT, and the loop runs 3 times.",
        proof: Proof::Count(&[TurnRight]),
        ..Q
    },
    Question {
        id: "w2-03",
        prompt: "How many steps forward does the miner take?",
        program: "[R3 [R2 F ] ]",
        choices: Choices::Text(&["6", "5", "3"]),
        explain: "The inside loop takes 2 steps. The outside loop runs it 3 times: 3 x 2 = 6.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w2-04",
        prompt: "How many steps forward does the miner take?",
        program: "[R4 [R3 F ] R ]",
        choices: Choices::Text(&["12", "7", "16"]),
        explain: "Each time round the big loop, the small loop takes 3 steps. 4 x 3 = 12. The TURN RIGHT doesn't count as a step.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w2-05",
        prompt: "Which loop does the same as this program?",
        program: "F F F F F F",
        choices: Choices::Programs(&["[R6 F ]", "[R5 F ]", "[R3 F F F ]"]),
        explain: "Six FORWARDs in a row is the same as repeating FORWARD 6 times. REPEAT 3 of three FORWARDs would be 9 steps.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w2-06",
        prompt: "Which loop does the same as this program?",
        program: "F R F R F R F R",
        choices: Choices::Programs(&["[R4 F R ]", "[R4 F ] [R4 R ]", "[R2 F R ]"]),
        explain: "The pattern \"forward, turn right\" happens 4 times, so put those two blocks inside REPEAT 4. The order inside the loop matters!",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w2-07",
        prompt: "Where does the miner end up?",
        program: "[R2 F F R ]",
        map: ROOM_7,
        choices: Choices::Spots(&[(3, 3), (1, 1), (5, 5)]),
        explain: "First time round: 2 steps up, then turn right. Second time: 2 steps right, then turn right again.",
        ..Q
    },
    Question {
        id: "w2-08",
        prompt: "Which program reaches the portal?",
        map: &["TTTTTTTTTT", "T>......PT", "TTTTTTTTTT"],
        choices: Choices::Programs(&["[R7 F ]", "[R6 F ]", "[R3 F F ]"]),
        explain: "The portal is 7 squares away. REPEAT 3 of two steps is only 6 steps, one short.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w2-09",
        prompt: "Which program reaches the portal?",
        map: &["TTTTTTTT", "T>#.#.PT", "TTTTTTTT"],
        choices: Choices::Programs(&["[R2 M F F ] F", "[R2 F M F ] F", "[R3 M F F ]"]),
        explain: "The pattern is \"mine, step, step\" twice, then one last step. Running the loop a third time tries to mine the portal!",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w2-10",
        prompt: "How many steps forward does the miner take?",
        program: "[R2 [R2 [R2 F ] ] ]",
        choices: Choices::Text(&["8", "6", "2"]),
        explain: "Three loops of 2, one inside the other: 2 x 2 x 2 = 8.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w2-11",
        prompt: "The backpack has 2 blocks. What happens?",
        program: "[R3 P F ] F",
        map: &["TTTTTTTT", "T>LLL.PT", "TTTTTTTT"],
        blocks: 2,
        choices: Choices::Text(&[
            "It runs out of blocks before the last lava",
            "It reaches the portal",
            "It steps into the first lava",
        ]),
        explain: "The loop wants to place 3 blocks, but there are only 2 in the backpack. The third PLACE fails.",
        proof: Proof::Ends(End::NoBlocks),
    },
    Question {
        id: "w2-12",
        prompt: "The backpack has 3 blocks. Which program reaches the portal?",
        map: &["TTTTTTTT", "T>LLL.PT", "TTTTTTTT"],
        blocks: 3,
        choices: Choices::Programs(&["[R3 P F ] F F", "[R3 F P ] F F", "[R2 P F ] F F"]),
        explain: "Place a bridge, step onto it, three times. Stepping first means stepping into lava, and 2 bridges aren't enough.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w2-13",
        prompt: "How many blocks run in total?",
        program: "[R5 F R ]",
        choices: Choices::Text(&["10", "5", "2"]),
        explain: "Each time round the loop, 2 blocks run (FORWARD and TURN RIGHT). 5 x 2 = 10.",
        proof: Proof::Count(ALL_ACTIONS),
        ..Q
    },
    Question {
        id: "w2-14",
        prompt: "Where does the miner end up?",
        program: "[R2 F L F R ]",
        map: &["TTTTTTT", "T.....T", "T.....T", "T.....T", "T>....T", "TTTTTTT"],
        choices: Choices::Spots(&[(3, 2), (5, 4), (1, 2)]),
        explain: "Each time round: one step right, turn to face up, one step up, turn back to face right. Two stair steps!",
        ..Q
    },
    Question {
        id: "w2-15",
        prompt: "Which program ends up exactly the same as this one?",
        program: "[R4 R ]",
        choices: Choices::Programs(&["[R2 L L ]", "[R2 R ]", "R"]),
        explain: "Four turns the same way spin the miner all the way round, back to where it started. Four left turns do that too.",
        proof: Proof::SameResult,
        ..Q
    },
];

// ============================ World 3: Functions ============================
const W3: &[Question] = &[
    Question {
        id: "w3-01",
        prompt: "How many steps forward does the miner take?",
        program: "C C C / F F",
        choices: Choices::Text(&["6", "2", "3"]),
        explain: "The function takes 2 steps, and MAIN calls it 3 times: 3 x 2 = 6.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w3-02",
        prompt: "How many times does the miner turn right?",
        program: "[R2 C ] / F R",
        choices: Choices::Text(&["2", "1", "4"]),
        explain: "The function has one TURN RIGHT. The loop calls the function 2 times.",
        proof: Proof::Count(&[TurnRight]),
        ..Q
    },
    Question {
        id: "w3-03",
        prompt: "How many steps forward does the miner take?",
        program: "C F C / [R3 F ]",
        choices: Choices::Text(&["7", "4", "5"]),
        explain: "Each call takes 3 steps, and there are 2 calls. Plus the 1 FORWARD in MAIN: 3 + 1 + 3 = 7.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w3-04",
        prompt: "How many steps forward does the miner take?",
        program: "[R3 C ] / [R2 F ]",
        choices: Choices::Text(&["6", "5", "3"]),
        explain: "The function takes 2 steps. MAIN's loop calls it 3 times: 3 x 2 = 6.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w3-05",
        prompt: "Where does the miner end up?",
        program: "C C C / F R",
        map: &["TTTTTTT", "T.....T", "T.....T", "T.....T", "T.^...T", "T.....T", "TTTTTTT"],
        choices: Choices::Spots(&[(3, 4), (2, 1), (5, 4)]),
        explain: "Each call is \"step, turn right\". Up, turn. Right, turn. Down, turn. Three calls trace three sides of a square.",
        ..Q
    },
    Question {
        id: "w3-06",
        prompt: "Which program reaches the portal?",
        map: &["TTTTTTTTT", "T>#.#.#PT", "TTTTTTTTT"],
        choices: Choices::Programs(&["C C C / M F F", "C C C / F M F", "C C / M F F"]),
        explain: "The function \"mine, step, step\" clears one stone and moves past it. There are 3 stones, so call it 3 times.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w3-07",
        prompt: "What happens when this program runs?",
        program: "C C / M F",
        map: &["TTTTTTT", "T>#..PT", "TTTTTTT"],
        choices: Choices::Text(&["The second MINE swings at thin air", "It reaches the portal", "It bumps into the stone"]),
        explain: "The first call mines the stone and steps forward. The second call tries to MINE again, but there's only empty ground ahead.",
        proof: Proof::Ends(End::NothingToMine),
        ..Q
    },
    Question {
        id: "w3-08",
        prompt: "Which program does exactly the same as this one?",
        program: "C C / L F R",
        choices: Choices::Programs(&["L F R L F R", "L F R", "L L F F R R"]),
        explain: "Calling a function runs all its blocks, in order. Two calls means the whole function twice.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w3-09",
        prompt: "How many steps forward does the miner take?",
        program: "[R2 C F ] / [R2 F ] R",
        choices: Choices::Text(&["6", "5", "4"]),
        explain: "Each time round the loop: the function takes 2 steps, then MAIN takes 1 more. That's 3, and the loop runs twice.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w3-10",
        prompt: "Which program collects both diamonds?",
        map: &["TTTTTTTT", "TTDTDTTT", "T>.....T", "TTTTTTTT"],
        choices: Choices::Programs(&["C F C / F L M R", "C C / F L M R", "C F C / F R M L"]),
        explain: "The function steps forward, faces up (to the miner's left) and mines. The extra FORWARD between the calls lines up the second diamond.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w3-11",
        prompt: "How many times does the function run?",
        program: "[R3 C ] C / F",
        choices: Choices::Text(&["4", "3", "1"]),
        explain: "The loop calls it 3 times, then there's one more CALL after the loop.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w3-12",
        prompt: "This program builds 4 bridges. How many PLACE BLOCKs run?",
        program: "C C C C / M F P F",
        map: &["TTTTTTTTTTTT", "T>#L#L#L#L.T", "TTTTTTTTTTTT"],
        choices: Choices::Text(&["4", "1", "8"]),
        explain: "There's only one PLACE BLOCK written down, but the function runs 4 times, so it places 4 blocks. Write it once, use it many times!",
        proof: Proof::Count(&[Place]),
        ..Q
    },
    Question {
        id: "w3-13",
        prompt: "Which program does exactly the same as this one?",
        program: "F F R F F R",
        choices: Choices::Programs(&["C C / F F R", "C / F F R", "C C / F R F"]),
        explain: "\"Forward, forward, turn right\" happens twice. Put it in the function once and call it twice.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w3-14",
        prompt: "Where does the miner end up?",
        program: "C L C / F F",
        map: &["TTTTTTT", "T.....T", "T.....T", "T.....T", "T.....T", "T>....T", "TTTTTTT"],
        choices: Choices::Spots(&[(3, 3), (5, 5), (3, 1)]),
        explain: "The first call takes 2 steps right. TURN LEFT faces up. The second call takes 2 steps up.",
        ..Q
    },
    Question {
        id: "w3-15",
        prompt: "The backpack has 1 block. What happens?",
        program: "C C F F / P F",
        map: &["TTTTTTT", "T>LL.PT", "TTTTTTT"],
        blocks: 1,
        choices: Choices::Text(&["It runs out of blocks", "It reaches the portal", "It steps into the lava"]),
        explain: "Each call places one block. The first call uses the only block, so the second call has nothing to place.",
        proof: Proof::Ends(End::NoBlocks),
    },
];

// ============================ World 4: If / Else ============================
const W4: &[Question] = &[
    Question {
        id: "w4-01",
        prompt: "The ? square might be stone or empty. Which program works every time?",
        map: &["TTTTTT", "T>?.PT", "TTTTTT"],
        choices: Choices::Programs(&["[I3 M ] F F F", "M F F F", "F F F"]),
        explain: "IF checks first and only mines when there really is stone. Always mining fails when it's empty; never mining bonks when it's stone.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w4-02",
        prompt: "What happens when this program runs?",
        program: "[I3 M ] F F F",
        map: &["TTTTTT", "T>#.PT", "TTTTTT"],
        choices: Choices::Text(&[
            "It mines the stone, then walks to the portal",
            "It skips the MINE and bonks into the stone",
            "It swings the pickaxe at thin air",
        ]),
        explain: "There IS stone ahead, so the IF check is true and the MINE inside runs.",
        proof: Proof::Ends(End::Wins),
        ..Q
    },
    Question {
        id: "w4-03",
        prompt: "What happens when this program runs?",
        program: "[I3 M ] F F F",
        map: &["TTTTTT", "T>..PT", "TTTTTT"],
        choices: Choices::Text(&[
            "It skips the MINE and walks to the portal",
            "It swings the pickaxe at thin air",
            "It bonks into stone",
        ]),
        explain: "There's no stone ahead, so the IF check is false and the MINE inside is skipped. Same program, different world, different result!",
        proof: Proof::Ends(End::Wins),
        ..Q
    },
    Question {
        id: "w4-04",
        prompt: "How many times does MINE actually run?",
        program: "[R6 [I3 M ] F ]",
        map: &["TTTTTTTTT", "T>#.#..PT", "TTTTTTTTT"],
        choices: Choices::Text(&["2", "6", "0"]),
        explain: "The IF is checked 6 times, but there's stone ahead only twice. MINE only runs when the check is true.",
        proof: Proof::Count(&[Mine]),
        ..Q
    },
    Question {
        id: "w4-05",
        prompt: "Each ! might be lava or empty. The backpack has 2 blocks. Which program works every time?",
        map: &["TTTTTTTT", "T>!.!.PT", "TTTTTTTT"],
        blocks: 2,
        choices: Choices::Programs(&["[R5 [I5 P ] F ]", "[R5 P F ]", "[R5 F ]"]),
        explain: "Only place a bridge IF there's lava ahead. Placing every time builds walls on empty ground; never placing walks into lava.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w4-06",
        prompt: "Each * is stone or lava. The backpack has 2 blocks. Which program works every time?",
        map: &["TTTTTTT", "T>*.*PT", "TTTTTTT"],
        blocks: 2,
        choices: Choices::Programs(&["[R2 [E3 M | P ] F F ]", "[R2 [E5 M | P ] F F ]", "[R2 M F F ]"]),
        explain: "IF stone ahead: mine it. ELSE it must be lava: bridge it. Checking for LAVA first gets it backwards and tries to mine lava.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w4-07",
        prompt: "Where does the miner end up?",
        program: "[E1 L | R ] F",
        map: &["TTTTT", "TTTTT", "T.^.T", "TTTTT"],
        choices: Choices::Spots(&[(1, 2), (3, 2), (2, 2)]),
        explain: "There IS a path to the left, so the IF part runs: turn left, then step forward. The ELSE part is skipped.",
        ..Q
    },
    Question {
        id: "w4-08",
        prompt: "Where does the miner end up?",
        program: "[E1 L | R ] F",
        map: &["TTTTT", "TTTTT", "TT^.T", "TTTTT"],
        choices: Choices::Spots(&[(3, 2), (2, 2), (2, 1)]),
        explain: "There's NO path to the left this time, so the ELSE part runs instead: turn right, then step forward.",
        ..Q
    },
    Question {
        id: "w4-09",
        prompt: "How many times does PLACE BLOCK run?",
        program: "[R4 [E3 M | P ] F F ]",
        map: &["TTTTTTTTTTT", "T>#.L.#.LPT", "TTTTTTTTTTT"],
        choices: Choices::Text(&["2", "4", "0"]),
        explain: "Every time round, IF/ELSE picks exactly one side. Stone twice (MINE) and lava twice (PLACE).",
        proof: Proof::Count(&[Place]),
        ..Q
    },
    Question {
        id: "w4-10",
        prompt: "Each $ might be a diamond or empty. Which program works every time?",
        map: &["TTTTTTT", "TT$$$TT", "T>...PT", "TTTTTTT"],
        choices: Choices::Programs(&["[R3 F L [I4 M ] R ] F", "[R3 F L M R ] F", "[R3 F R [I4 M ] L ] F"]),
        explain: "Look up (the miner's left) and mine only IF there's a diamond. Turning RIGHT first looks the wrong way and never finds them.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w4-11",
        prompt: "How many times does MINE run?",
        program: "[R3 [I3 M ] F ]",
        map: &["TTTTTTT", "T>...PT", "TTTTTTT"],
        choices: Choices::Text(&["0", "3", "1"]),
        explain: "There's never stone ahead, so the IF check is false every time and MINE never runs.",
        proof: Proof::Count(&[Mine]),
        ..Q
    },
    Question {
        id: "w4-12",
        prompt: "In this world, which program does exactly the same as this one?",
        program: "[E0 F | R ]",
        map: &["TTTT", "T>.T", "TTTT"],
        choices: Choices::Programs(&["F", "R", "F R"]),
        explain: "There's a path ahead, so the IF part (FORWARD) runs and the ELSE part (TURN RIGHT) doesn't.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w4-13",
        prompt: "Each ~ might be water or empty. The backpack has 2 blocks. Which program works every time?",
        map: &["TTTTTT", "T>~~PT", "TTTTTT"],
        blocks: 2,
        choices: Choices::Programs(&["[R3 [I6 P ] F ]", "[R3 [I5 P ] F ]", "[R3 P F ]"]),
        explain: "Check for WATER, not lava! The program that checks for lava asks the wrong question, so it never builds a bridge.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w4-14",
        prompt: "Where does the miner end up?",
        program: "[E0 F | [E1 L | R ] ] F",
        map: &["TTTTT", "TTTTT", "TT^.T", "TTTTT"],
        choices: Choices::Spots(&[(3, 2), (1, 2), (2, 1)]),
        explain: "No path ahead, so go to ELSE. No path left either, so the inner ELSE runs: turn right. Then step forward.",
        ..Q
    },
    Question {
        id: "w4-15",
        prompt: "How many times does the miner turn?",
        program: "[R4 [E0 F | R ] ]",
        map: &["TTTT", "T>.T", "TT.T", "TTTT"],
        choices: Choices::Text(&["2", "4", "1"]),
        explain: "Round 1: path ahead, step. Round 2: wall, turn. Round 3: path, step. Round 4: wall, turn. 2 turns.",
        proof: Proof::Count(TURNS),
        ..Q
    },
];

// ===================== World 5: While loops & algorithms =====================
const W5: &[Question] = &[
    Question {
        id: "w5-01",
        prompt: "How many steps does the miner take?",
        program: "[W0 F ]",
        map: &["TTTTTTTT", "T>.....T", "TTTTTTTT"],
        choices: Choices::Text(&["5", "6", "1"]),
        explain: "WHILE keeps going as long as there's a path ahead. After 5 steps the miner faces the wall, so the loop stops.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w5-02",
        prompt: "The portal is on one of the p squares. Which program works every time?",
        map: &["TTTTTTTT", "T>..pppT", "TTTTTTTT"],
        choices: Choices::Programs(&["[W7 F ]", "[R3 F ]", "F F F F"]),
        explain: "WHILE NOT AT PORTAL keeps walking until it gets there, however far that is. Counting steps only works if you know the distance.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w5-03",
        prompt: "What happens when this program runs?",
        program: "[W0 F ] R F",
        map: &["TTTTTT", "T>...T", "TTTTPT"],
        choices: Choices::Text(&[
            "It walks to the wall, turns, and steps into the portal",
            "It takes one step and stops",
            "It walks forever",
        ]),
        explain: "The WHILE loop walks until the wall. Then the blocks AFTER the loop run: turn right and step into the portal.",
        proof: Proof::Ends(End::Wins),
        ..Q
    },
    Question {
        id: "w5-04",
        prompt: "How many times does MINE run?",
        program: "[W3 M F ] F",
        map: &["TTTTTTT", "T>###PT", "TTTTTTT"],
        choices: Choices::Text(&["3", "4", "1"]),
        explain: "The loop runs while there's stone ahead. There are 3 stones, then the portal, so it mines 3 times.",
        proof: Proof::Count(&[Mine]),
        ..Q
    },
    Question {
        id: "w5-05",
        prompt: "The backpack has 3 blocks. Which program reaches the portal?",
        map: &["TTTTTTTT", "T>LLL.PT", "TTTTTTTT"],
        blocks: 3,
        choices: Choices::Programs(&["[W5 P F ] F F", "[W0 F ]", "[W5 F P ] F F"]),
        explain: "WHILE LAVA AHEAD: bridge it and step on. Lava isn't a path, so WHILE PATH AHEAD never even starts.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w5-06",
        prompt: "Which program finds the portal?",
        map: &["TTTTTTT", "T>..TTT", "TTT.TTT", "TTT..PT", "TTTTTTT"],
        choices: Choices::Programs(&["[W7 [E2 R F | [E0 F | L ] ] ]", "[W7 [E0 F | R ] ]", "[W0 F ]"]),
        explain: "Keep your right hand on the wall: turn right when you can, else go forward, else turn left. Only turning right gets stuck going back and forth.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w5-07",
        prompt: "How many times does the miner turn?",
        program: "[W7 [E0 F | R ] ]",
        map: &["TTTTT", "T>..T", "TTT.T", "TP..T", "TTTTT"],
        choices: Choices::Text(&["2", "3", "4"]),
        explain: "It walks until a wall, turns right (down), walks, hits a wall, turns right again (left), and walks into the portal.",
        proof: Proof::Count(TURNS),
        ..Q
    },
    Question {
        id: "w5-08",
        prompt: "What happens when this program runs?",
        program: "[W7 R ]",
        map: &["TTTTTT", "T>..PT", "TTTTTT"],
        choices: Choices::Text(&["It spins on the spot forever", "It walks to the portal", "It turns once and stops"]),
        explain: "The loop only stops when the miner is at the portal, but turning never moves it there. The check is never false!",
        proof: Proof::Ends(End::Stuck),
        ..Q
    },
    Question {
        id: "w5-09",
        prompt: "On THIS map, which program does the same as the WHILE loop?",
        program: "[W0 F ]",
        map: &["TTTTTTT", "T>....T", "TTTTTTT"],
        choices: Choices::Programs(&["[R4 F ]", "[R5 F ]", "[R3 F ]"]),
        explain: "There are 4 empty squares, so WHILE runs 4 times here. On a longer road REPEAT 4 would stop short, but WHILE would keep going.",
        proof: Proof::SameResult,
        ..Q
    },
    Question {
        id: "w5-10",
        prompt: "Which program reaches the portal?",
        map: &["TTTTTTT", "T>##.PT", "TTTTTTT"],
        choices: Choices::Programs(&["[W3 M F ] F F", "[R3 M F ] F", "[W0 F ] F"]),
        explain: "WHILE STONE AHEAD mines exactly as many stones as there are. REPEAT 3 tries a third MINE on empty ground.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w5-11",
        prompt: "How many times does MINE run?",
        program: "[W7 [I3 M ] F ]",
        map: &["TTTTTTTT", "T>.#.#PT", "TTTTTTTT"],
        choices: Choices::Text(&["2", "3", "5"]),
        explain: "Each time round, the IF checks for stone. There are 2 stones on the way, so MINE runs twice.",
        proof: Proof::Count(&[Mine]),
        ..Q
    },
    Question {
        id: "w5-12",
        prompt: "Where does the miner end up?",
        program: "[W0 F ] L [W0 F ]",
        map: &["TTTTT", "T...T", "T...T", "T>..T", "TTTTT"],
        choices: Choices::Spots(&[(3, 1), (1, 1), (3, 3)]),
        explain: "Walk right until the wall, turn left to face up, then walk up until the next wall.",
        ..Q
    },
    Question {
        id: "w5-13",
        prompt: "The portal is on one of the p squares. Which program works every time?",
        map: &["TTTTTT", "T>..pT", "TTTTpT", "TTTTpT", "TTTTTT"],
        choices: Choices::Programs(&["[W7 [E0 F | R ] ]", "[W0 F ]", "[W7 [E0 F | L ] ]"]),
        explain: "At the corner the path goes DOWN, which is to the miner's right. Turning left faces a wall, and then it walks back and forth forever.",
        proof: Proof::OnlyWinner,
        ..Q
    },
    Question {
        id: "w5-14",
        prompt: "How many steps forward does the miner take?",
        program: "[R3 [W0 F ] R ]",
        map: &["TTTTT", "T...T", "T...T", "T>..T", "TTTTT"],
        choices: Choices::Text(&["4", "6", "9"]),
        explain: "Round 1: 2 steps to the wall, turn. Round 2: facing a wall already, so 0 steps, turn. Round 3: 2 steps back. 2 + 0 + 2 = 4.",
        proof: Proof::Count(&[Forward]),
        ..Q
    },
    Question {
        id: "w5-15",
        prompt: "The backpack has 2 blocks. What happens?",
        program: "[W7 [I5 P ] F ]",
        map: &["TTTTTTTT", "T>.L.LPT", "TTTTTTTT"],
        blocks: 2,
        choices: Choices::Text(&[
            "It bridges both lava pools and reaches the portal",
            "It steps into the first lava",
            "It runs out of blocks",
        ]),
        explain: "Every time round, IF LAVA AHEAD places a bridge, then the miner steps. 2 lava pools, 2 blocks: just enough.",
        proof: Proof::Ends(End::Wins),
    },
];

/// One bank per World Check. Check `c` comes after world `c` (0-based), so
/// there is one fewer bank than there are worlds: the last world has no
/// world after it to unlock.
pub const BANKS: [&[Question]; 5] = [W1, W2, W3, W4, W5];
