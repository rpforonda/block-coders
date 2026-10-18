//! All the levels. Each chapter introduces one big programming idea and
//! starts easy, getting harder level by level.
//!
//! See `World::from_level` for what each map symbol means. Run `cargo test`
//! after changing a level: the tests check that every `solution` wins in
//! every random version of its level.

use crate::program::{Cond, Kind};
use crate::world::Floor;

pub struct Chapter {
    pub name: &'static str,
    pub concept: &'static str,
    pub color: (u8, u8, u8),
}

pub const CHAPTERS: [Chapter; 6] = [
    Chapter { name: "Grassy Plains", concept: "Sequences", color: (98, 163, 58) },
    Chapter { name: "Loop Caves", concept: "Loops", color: (150, 110, 75) },
    Chapter { name: "Function Forest", concept: "Functions", color: (40, 120, 60) },
    Chapter { name: "Decision Desert", concept: "If / Else", color: (210, 180, 90) },
    Chapter { name: "The Deep Dark", concept: "While loops & algorithms", color: (60, 70, 110) },
    Chapter { name: "Nether Fortress", concept: "Master challenges", color: (170, 50, 40) },
];

pub struct Level {
    /// Also the level's save key, so keep names unique (and free of `;`).
    pub name: &'static str,
    pub chapter: usize,
    /// One map is picked at random each time the world is built.
    pub maps: &'static [&'static [&'static str]],
    pub palette: &'static [Kind],
    pub conds: &'static [Cond],
    /// Blocks needed for three stars.
    pub par: u32,
    pub max_blocks: Option<u32>,
    pub start_blocks: u32,
    /// Shown when the level opens: (title, explanation).
    pub intro: Option<(&'static str, &'static str)>,
    pub hint: &'static str,
    /// Shown after winning: the programming idea they just used.
    pub lesson: &'static str,
    pub floor: Floor,
    /// A working solution (in the save format). Used by tests, and by
    /// parents who press Ctrl+Shift+S on a level.
    pub solution: &'static str,
}

impl Level {
    pub fn is_random(&self) -> bool {
        self.maps.len() > 1
            || self.maps.iter().any(|m| m.iter().any(|row| row.contains(['?', '!', '*', '$', '~', '&', 'p'])))
    }
}

/// Level order from before there were 60 levels, so old saves still load.
pub const LEGACY_ORDER: [&str; 20] = [
    "First Steps", "Turn the Corner", "Emerald Trail", "Rock Blocks", "Diamond Hunter", "Lava Bridge",
    "The Long Hallway", "Tunnel Digger", "Crystal Staircase", "Around the Pond", "Bridge Builder",
    "Diamond Rows", "Mine Shafts", "Surprise Stones", "Stone or Lava?", "Diamond Detector", "How Far?",
    "Winding Road", "The Maze", "Deep Dark Gauntlet",
];

use Kind::*;

const P_MOVE: &[Kind] = &[Forward];
const P_TURN: &[Kind] = &[Forward, TurnLeft, TurnRight];
const P_MINE: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine];
const P_PLACE: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place];
const P_LOOP: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Repeat];
const P_FUNC_ONLY: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Call];
const P_FUNC: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Repeat, Call];
const P_IF: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Repeat, Call, If];
const P_IFELSE: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Repeat, Call, If, IfElse];
const P_ALL: &[Kind] = &[Forward, TurnLeft, TurnRight, Mine, Place, Repeat, Call, If, IfElse, While];

const NO_CONDS: &[Cond] = &[];
const STONE_CONDS: &[Cond] = &[Cond::StoneAhead, Cond::PathAhead];
const HAZARD_CONDS: &[Cond] = &[Cond::StoneAhead, Cond::LavaAhead, Cond::WaterAhead, Cond::PathAhead];
const GEM_CONDS: &[Cond] = &[Cond::DiamondAhead, Cond::StoneAhead, Cond::PathAhead];
const TURN_CONDS: &[Cond] = &[Cond::PathAhead, Cond::PathLeft, Cond::PathRight];
const PATH_CONDS: &[Cond] = &[Cond::PathAhead, Cond::PathLeft, Cond::PathRight, Cond::NotAtPortal];
const WALK_CONDS: &[Cond] = &[Cond::PathAhead, Cond::StoneAhead, Cond::LavaAhead, Cond::NotAtPortal];
const ALL_CONDS: &[Cond] = &[
    Cond::PathAhead,
    Cond::PathLeft,
    Cond::PathRight,
    Cond::StoneAhead,
    Cond::DiamondAhead,
    Cond::LavaAhead,
    Cond::WaterAhead,
    Cond::NotAtPortal,
];

/// Fills in the fields most levels share, so each level below only lists
/// what makes it special.
pub const BASE: Level = Level {
    name: "",
    chapter: 0,
    maps: &[],
    palette: P_ALL,
    conds: NO_CONDS,
    par: 0,
    max_blocks: None,
    start_blocks: 0,
    intro: None,
    hint: "",
    lesson: "",
    floor: Floor::Grass,
    solution: "",
};

pub static LEVELS: &[Level] = &[
    // ======================= World 1: Sequences =======================
    Level {
        name: "First Steps",
        maps: &[&["TTTTTTT", "T>...PT", "TTTTTTT"]],
        palette: P_MOVE,
        par: 4,
        intro: Some((
            "Welcome, Coder!",
            "Hi, I'm Bit! Computers follow instructions EXACTLY, one at a time, in order. \
             A list of instructions is called a PROGRAM.\n\n\
             Click the blue FORWARD block to add it to your program on the right. \
             Then press RUN to walk the miner into the purple portal!",
        )),
        hint: "Count the grass squares between the miner and the portal. You need one FORWARD for each square.",
        lesson: "You wrote your first program! Steps that run one after another, in order, are called a SEQUENCE.",
        solution: "F F F F",
        ..BASE
    },
    Level {
        name: "Turn the Corner",
        maps: &[&["TTTTTTT", "T>..TTT", "TTT.TTT", "TTT..PT", "TTTTTTT"]],
        palette: P_TURN,
        par: 8,
        intro: Some((
            "New blocks: TURN LEFT and TURN RIGHT",
            "Turning does NOT move the miner - it only changes which way they face. \
             The yellow arrow shows which way the miner is looking.\n\n\
             Tip: pretend YOU are the miner. Which hand points the way you need to go?",
        )),
        hint: "Walk to the corner, turn RIGHT to face down, walk down, then turn LEFT to face the portal.",
        lesson: "Turning changes direction. Thinking about which way you're facing is a big part of programming robots!",
        solution: "F F R F F L F F",
        ..BASE
    },
    Level {
        name: "Emerald Trail",
        maps: &[&["TTTTTTTTT", "T>.E.TTTT", "TTTT.TTTT", "TTTT.E.PT", "TTTTTTTTT"]],
        palette: P_TURN,
        par: 10,
        intro: Some((
            "Treasure!",
            "Green emeralds are treasure. Walk over them to pick them up.\n\n\
             The portal only works once you've collected ALL the treasure, so plan your path!",
        )),
        hint: "Trace the path with your finger first. Say each step out loud: forward, forward, ...",
        lesson: "Great planning! Programmers often trace through their code step by step to check it - just like you did.",
        solution: "F F F R F F L F F F",
        ..BASE
    },
    Level {
        name: "Zig Zag",
        maps: &[&["TTTTTTTT", "T>.TTTTT", "TT.E.TTT", "TTTTE.PT", "TTTTTTTT"]],
        palette: P_TURN,
        par: 11,
        hint: "Right, then left, then right, then left... After each turn, count how many steps to the next corner.",
        lesson: "You used both turns in a long sequence. Tracking direction in your head is real programmer brain-work!",
        solution: "F R F L F F R F L F F",
        ..BASE
    },
    Level {
        name: "Rock Blocks",
        maps: &[&["TTTTTTTT", "T>.#.#PT", "TTTTTTTT"]],
        palette: P_MINE,
        par: 7,
        intro: Some((
            "New block: MINE",
            "Stone in the way? MINE breaks the block right in front of the miner. \
             Each stone you mine goes into your backpack.\n\n\
             Careful: mining doesn't move you. You still need FORWARD to step into the gap!",
        )),
        hint: "Mine only works on the block directly in front of you. Walk up to the stone first.",
        lesson: "Mining and walking are different instructions. Computers need every single step spelled out!",
        solution: "F M F F M F F",
        ..BASE
    },
    Level {
        name: "Mine and Turn",
        maps: &[&["TTTTTTT", "T>#.TTT", "TTT#TTT", "TTT.#PT", "TTTTTTT"]],
        palette: P_MINE,
        par: 11,
        hint: "Each stone needs: mine, forward. Then one more forward to the corner, and turn.",
        lesson: "Did you notice 'mine, forward, forward' happened three times? Remember that - patterns will be useful soon!",
        solution: "M F F R M F F L M F F",
        ..BASE
    },
    Level {
        name: "Diamond Hunter",
        maps: &[&["TTTTTTTTT", "TTTTDTTTT", "T>D..D.PT", "TTTTTTTTT"]],
        palette: P_MINE,
        par: 11,
        intro: Some((
            "Diamonds!",
            "Diamond ore is stone with sparkly blue gems in it. MINE it to collect the diamond.\n\n\
             You need ALL the diamonds before the portal will light up. One is hiding up above the path!",
        )),
        hint: "To mine the diamond above the path, stand under it, turn to face it, mine, then turn back.",
        lesson: "You broke a big task into small steps. That's called DECOMPOSITION - a real programmer skill!",
        solution: "M F F F L M R M F F F",
        ..BASE
    },
    Level {
        name: "Lava Bridge",
        maps: &[&["TTTTTTTTT", "T>##LL.PT", "TTTTTTTTT"]],
        palette: P_PLACE,
        par: 10,
        intro: Some((
            "New block: PLACE BLOCK",
            "HOT LAVA! Never walk into it.\n\n\
             PLACE BLOCK takes a block out of your backpack and puts it in front of you. \
             Placing a block on lava or water turns it into a safe bridge.\n\n\
             Your backpack starts empty - mine some stone first!",
        )),
        hint: "Mine both stones to get 2 blocks. Then place one, step forward, place the other, step forward.",
        lesson: "You saved blocks in your backpack and used them later. Programs store information to use later too - in VARIABLES.",
        solution: "M F M F P F P F F F",
        ..BASE
    },
    Level {
        name: "Water Crossing",
        maps: &[&["TTTTTT", "T>##TT", "TTTWTT", "TTTWTT", "TTT.PT", "TTTTTT"]],
        palette: P_PLACE,
        par: 12,
        hint: "Mine both stones first so your backpack has 2 blocks. Then turn and bridge the water one square at a time.",
        lesson: "You planned ahead: collect first, use later. Good programs often gather what they need before they start!",
        solution: "M F M F R P F P F F L F",
        ..BASE
    },
    // ======================= World 2: Loops =======================
    Level {
        name: "The Long Hallway",
        chapter: 1,
        maps: &[&["BBBBBBBBBBBBB", "B>.........PB", "BBBBBBBBBBBBB"]],
        palette: P_LOOP,
        par: 2,
        max_blocks: Some(3),
        intro: Some((
            "New block: REPEAT",
            "Whoa, that's a long hallway... and you only have room for 3 blocks!\n\n\
             A REPEAT block runs the blocks INSIDE it again and again. Add a REPEAT, \
             then add FORWARD while the yellow line is inside it. \
             Click the - and + buttons to choose how many times it repeats.",
        )),
        hint: "Count the squares to the portal, and set the REPEAT number to match.",
        lesson: "That's a LOOP! With loops, a tiny program can do a HUGE amount of work.",
        floor: Floor::Dirt,
        solution: "[R10 F ]",
        ..BASE
    },
    Level {
        name: "Around the Bend",
        chapter: 1,
        maps: &[&["BBBBBBBB", "B>.....B", "BBBBBB.B", "BBBBBB.B", "BBBBBBPB", "BBBBBBBB"]],
        palette: P_LOOP,
        par: 5,
        max_blocks: Some(6),
        hint: "Use one REPEAT for the long part, turn, then another REPEAT for the part going down.",
        lesson: "You can use more than one loop in a program - one for each part of the job.",
        floor: Floor::Dirt,
        solution: "[R5 F ] R [R3 F ]",
        ..BASE
    },
    Level {
        name: "Tunnel Digger",
        chapter: 1,
        maps: &[&["BBBBBBBBBBB", "B>#######PB", "BBBBBBBBBBB"]],
        palette: P_LOOP,
        par: 4,
        max_blocks: Some(5),
        hint: "Each stone needs the same two steps: MINE, then FORWARD. Put both inside a REPEAT.",
        lesson: "Loops can repeat more than one instruction. 'Mine, forward' was a PATTERN that repeated.",
        floor: Floor::Dirt,
        solution: "[R7 M F ] F",
        ..BASE
    },
    Level {
        name: "Bridge Builder",
        chapter: 1,
        maps: &[&["BBBBBBBBBBB", "B>WWWWWW.PB", "BBBBBBBBBBB"]],
        palette: P_LOOP,
        par: 5,
        max_blocks: Some(7),
        start_blocks: 6,
        hint: "Your backpack already has 6 blocks. Build one piece of bridge, step onto it, and repeat!",
        lesson: "You used a loop to BUILD something, piece by piece. That's how games create whole worlds with code!",
        floor: Floor::Dirt,
        solution: "[R6 P F ] F F",
        ..BASE
    },
    Level {
        name: "Dig and Bridge",
        chapter: 1,
        maps: &[&["BBBBBBBBBBBBB", "B>####LLLL.PB", "BBBBBBBBBBBBB"]],
        palette: P_LOOP,
        par: 8,
        max_blocks: Some(10),
        hint: "First loop: mine and step through the stone. That fills your backpack! Second loop: place and step across the lava.",
        lesson: "The first loop collected what the second loop needed. Programs often work in stages like this.",
        floor: Floor::Dirt,
        solution: "[R4 M F ] [R4 P F ] F F",
        ..BASE
    },
    Level {
        name: "Crystal Staircase",
        chapter: 1,
        maps: &[&["BBBBBBBB", "B>EBBBBB", "BBEEBBBB", "BBBEEBBB", "BBBBEEBB", "BBBBBEPB", "BBBBBBBB"]],
        palette: P_LOOP,
        par: 6,
        max_blocks: Some(8),
        hint: "Look for the pattern: forward, turn right, forward, turn left... How many times does it happen?",
        lesson: "You spotted the PATTERN! Finding patterns is one of the biggest superpowers a programmer has.",
        floor: Floor::Dirt,
        solution: "[R4 F R F L ] F",
        ..BASE
    },
    Level {
        name: "Rocky Stairs",
        chapter: 1,
        maps: &[&["BBBBBBBB", "B>#BBBBB", "BB##BBBB", "BBB##BBB", "BBBB##BB", "BBBBB#PB", "BBBBBBBB"]],
        palette: P_LOOP,
        par: 8,
        max_blocks: Some(10),
        hint: "It's the staircase again, but every step is stone. What do you need to add before each FORWARD?",
        lesson: "You changed an old pattern to fit a new problem. Programmers reuse ideas like this all the time!",
        floor: Floor::Dirt,
        solution: "[R4 M F R M F L ] F",
        ..BASE
    },
    Level {
        name: "Mountain Pass",
        chapter: 1,
        maps: &[&["BBBBBBBBBB", "BBBBEEBBBB", "BBB....BBB", "BB..BB..BB", "B>.BBBB.PB", "BBBBBBBBBB"]],
        palette: P_LOOP,
        par: 11,
        max_blocks: Some(13),
        hint: "Going up is one pattern (forward, left, forward, right). Going down is a different one. Use two loops!",
        lesson: "Two different patterns, two loops. Breaking a problem into parts makes it much easier to solve.",
        floor: Floor::Dirt,
        solution: "[R3 F L F R ] [R3 F R F L ] F",
        ..BASE
    },
    Level {
        name: "Around the Pond",
        chapter: 1,
        maps: &[&["BBBBBBB", "B>...EB", "BPWWW.B", "B.WWW.B", "B.WWW.B", "BE...EB", "BBBBBBB"]],
        palette: P_LOOP,
        par: 4,
        max_blocks: Some(6),
        intro: Some((
            "Loops inside loops",
            "You can put a REPEAT inside another REPEAT! That's called a NESTED LOOP.\n\n\
             Walking along one side of the pond is a pattern... and walking all four sides is a pattern too!",
        )),
        hint: "One side of the pond is: forward 4 times, then turn right. Do that 4 times.",
        lesson: "NESTED LOOPS - a loop inside a loop. Programmers use them to draw grids, sort lists, and build game maps.",
        floor: Floor::Dirt,
        solution: "[R4 [R4 F ] R ]",
        ..BASE
    },
    Level {
        name: "Snake Tunnel",
        chapter: 1,
        maps: &[&["BBBBBBBBBB", "B>###BBBBB", "BBBB#BBBBB", "BBBB####BB", "BBBBBBB#BB", "BBBBBBB#PB", "BBBBBBBBBB"]],
        palette: P_LOOP,
        par: 10,
        max_blocks: Some(12),
        hint: "Dig 3 across, turn right, dig 2 down, turn left. That whole thing happens twice!",
        lesson: "A loop full of loops! Nested loops let you describe big, repeating shapes with very little code.",
        floor: Floor::Dirt,
        solution: "[R2 [R3 M F ] R [R2 M F ] L ] F",
        ..BASE
    },
    Level {
        name: "Emerald Field",
        chapter: 1,
        maps: &[&["BBBBBBB", "B>EEEEB", "BEEEEEB", "BEEEEEB", "BEEEEEB", "BPBBBBB", "BBBBBBB"]],
        palette: P_LOOP,
        par: 11,
        max_blocks: Some(14),
        hint: "Mow it like a lawn: across, down one, back across, down one... Two rows make one pattern. Do it twice!",
        lesson: "This back-and-forth path is how robot vacuums and lawn mowers cover a whole area. You programmed a robot pattern!",
        floor: Floor::Dirt,
        solution: "[R2 [R4 F ] R F R [R4 F ] L F L ]",
        ..BASE
    },
    // ======================= World 3: Functions =======================
    Level {
        name: "Teach a New Word",
        chapter: 2,
        maps: &[&["TTTTTTTT", "T>####PT", "TTTTTTTT"]],
        palette: P_FUNC_ONLY,
        par: 7,
        max_blocks: Some(7),
        intro: Some((
            "New: FUNCTIONS",
            "A FUNCTION is like teaching the computer a brand new word.\n\n\
             Click the FUNCTION tab above your program and put MINE and FORWARD there. \
             Then, in MAIN, use CALL FUNCTION to run both steps with just one block!\n\n\
             (No REPEAT in this world's first levels - functions only!)",
        )),
        hint: "Function: mine, forward. Main: call it 4 times, then one more forward.",
        lesson: "You taught the computer a new word! Calling a FUNCTION runs all of its steps.",
        solution: "C C C C F / M F",
        ..BASE
    },
    Level {
        name: "Diamond Rows",
        chapter: 2,
        maps: &[&["TTTTTTTTTTT", "TTTDTDTDTTT", "T>.......PT", "TTTDTDTDTTT", "TTTTTTTTTTT"]],
        palette: P_FUNC_ONLY,
        par: 13,
        max_blocks: Some(16),
        hint: "Make the function: forward, forward, turn left, mine, turn right, turn right, mine, turn left. Then CALL it 3 times!",
        lesson: "FUNCTIONS let you write something once and use it again and again. Real apps are built from thousands of them!",
        solution: "C C C F F / F F L M R R M L",
        ..BASE
    },
    Level {
        name: "Bridge Spell",
        chapter: 2,
        maps: &[&["TTTTTTTTTTTTTT", "T>.LL..LL.LLPT", "TTTTTTTTTTTTTT"]],
        palette: P_FUNC,
        par: 11,
        max_blocks: Some(14),
        start_blocks: 6,
        hint: "Every lava pool is 2 squares. Make a function that builds a 2-square bridge and walks across it.",
        lesson: "Your function is a tool, and loops can live inside functions too!",
        solution: "F C F F C F C F / [R2 P F ]",
        ..BASE
    },
    Level {
        name: "Treasure Rooms",
        chapter: 2,
        maps: &[&["TTTTTTTTTT", "TTETETTETT", "TT.T.TT.TT", "T>......PT", "TTTTTTTTTT"]],
        palette: P_FUNC,
        par: 17,
        max_blocks: Some(20),
        hint: "Function: go into a room (turn left, 2 forward), turn around, come back out (2 forward), and face the hallway again.",
        lesson: "A function that ends facing the same way it started is easy to reuse anywhere. That's good design!",
        solution: "F C [R2 F ] C [R3 F ] C F / L [R2 F ] R R [R2 F ] L",
        ..BASE
    },
    Level {
        name: "Stone Bridges",
        chapter: 2,
        maps: &[&["TTTTTTTTTTTTT", "T>#L.#L..#LPT", "TTTTTTTTTTTTT"]],
        palette: P_FUNC,
        par: 11,
        max_blocks: Some(14),
        hint: "Each stone is followed by lava. Function: mine the stone, step, place a block on the lava, step.",
        lesson: "Your function both COLLECTS a block and USES it. Functions can do whole little jobs on their own.",
        solution: "C F C F F C F / M F P F",
        ..BASE
    },
    Level {
        name: "Mine Shafts",
        chapter: 2,
        maps: &[&["TTTTTTTTTTT", "TTDTDTTDTTT", "TT#T#TT#TTT", "T>.......PT", "TTTTTTTTTTT"]],
        palette: P_FUNC,
        par: 18,
        max_blocks: Some(22),
        hint: "The shafts are NOT evenly spaced, so a function works better than a loop here. \
               The function should dig up the shaft, grab the diamond, and come back down facing the same way.",
        lesson: "A function is a reusable tool. You used it in different places, with different code in between.",
        solution: "F C F F C [R3 F ] C F F / L M F M R R F L",
        ..BASE
    },
    Level {
        name: "Twin Tunnels",
        chapter: 2,
        maps: &[&[
            "TTTTTTTTT", "TTDDDDTTT", "T>.....TT", "TTTTTT.DT", "TTTTTT.DT", "TTTTTT.DT", "TTTTTTPTT", "TTTTTTTTT",
        ]],
        palette: P_FUNC,
        par: 11,
        max_blocks: Some(14),
        hint: "Both tunnels have diamonds on the miner's LEFT side. Function: step, turn left, mine, turn right. Call it in a loop!",
        lesson: "The same function worked in two different directions, because 'left' always means the miner's left.",
        solution: "[R4 C ] F R [R3 C ] F / F L M R",
        ..BASE
    },
    Level {
        name: "Up and Down",
        chapter: 2,
        maps: &[&[
            "TTTTTTTTTTTT", "TTDTTTDTTTTT", "TT#TTT#TTTTT", "T>........PT", "TTTT#TTT#TTT", "TTTTDTTTDTTT",
            "TTTTTTTTTTTT",
        ]],
        palette: P_FUNC,
        par: 20,
        max_blocks: Some(26),
        hint: "Make a function that digs the shaft on your LEFT. For shafts going down, turn around first - then down is on your left!",
        lesson: "You reused one function in a clever new way by changing which way you faced. That's creative coding!",
        solution: "[R2 F C F F R R C R R F ] F / L M F M R R F L",
        ..BASE
    },
    // ======================= World 4: If / Else =======================
    Level {
        name: "Maybe a Stone",
        chapter: 3,
        maps: &[&["TTTTTTT", "T>.?.PT", "TTTTTTT"]],
        palette: P_IF,
        conds: STONE_CONDS,
        par: 6,
        max_blocks: Some(8),
        intro: Some((
            "New block: IF",
            "See the dice button? This level CHANGES every time you play! Sometimes there's a stone, sometimes not.\n\n\
             An IF block checks something first. The blocks inside it ONLY run when the check is TRUE. \
             'If stone ahead: mine' only mines when there really is a stone.",
        )),
        hint: "Forward. IF stone ahead: mine. Then forward 3 times. Press the dice to test both worlds!",
        lesson: "Programs make DECISIONS with IF. Your code now works in every version of the world!",
        floor: Floor::Sand,
        solution: "F [I3 M ] F F F",
        ..BASE
    },
    Level {
        name: "Two Surprises",
        chapter: 3,
        maps: &[&["TTTTTTT", "T>?.?PT", "TTTTTTT"]],
        palette: P_IF,
        conds: STONE_CONDS,
        par: 5,
        max_blocks: Some(7),
        hint: "Check for a stone, then walk 2 squares. That happens twice!",
        lesson: "You put an IF inside a loop. The loop repeats, and the IF decides fresh each time.",
        floor: Floor::Sand,
        solution: "[R2 [I3 M ] F F ]",
        ..BASE
    },
    Level {
        name: "Surprise Stones",
        chapter: 3,
        maps: &[&["TTTTTTTTTTTTT", "T>?????????PT", "TTTTTTTTTTTTT"]],
        palette: P_IF,
        conds: STONE_CONDS,
        par: 4,
        max_blocks: Some(6),
        hint: "Repeat 10 times: IF there's stone ahead, mine it. Then move forward (outside the IF!).",
        lesson: "Ten different checks, one tiny program. Loops + IF make code that handles anything!",
        floor: Floor::Sand,
        solution: "[R10 [I3 M ] F ]",
        ..BASE
    },
    Level {
        name: "Lava Maybe",
        chapter: 3,
        maps: &[&["TTTTTTTTTT", "T>!!!!!!PT", "TTTTTTTTTT"]],
        palette: P_IF,
        conds: HAZARD_CONDS,
        par: 4,
        max_blocks: Some(6),
        start_blocks: 6,
        hint: "Only place a block where there's lava. Placing on plain ground makes a wall you'll bonk into!",
        lesson: "Checking before acting keeps the miner safe. Real programs check things before doing anything risky.",
        floor: Floor::Sand,
        solution: "[R7 [I5 P ] F ]",
        ..BASE
    },
    Level {
        name: "Puddles",
        chapter: 3,
        maps: &[&["TTTTTTTTTTTT", "T>?~?~?~?~PT", "TTTTTTTTTTTT"]],
        palette: P_IF,
        conds: HAZARD_CONDS,
        par: 6,
        max_blocks: Some(8),
        start_blocks: 4,
        intro: Some((
            "Two checks",
            "Some squares might be stone, some might be water - or neither!\n\n\
             You can use TWO IF blocks one after the other: one checks for stone, the other checks for water. \
             Click the white label on an IF block to change what it checks.",
        )),
        hint: "Repeat 9 times: IF stone ahead, mine. IF water ahead, place. Then forward.",
        lesson: "Several IFs in a row let a program handle lots of different situations.",
        floor: Floor::Sand,
        solution: "[R9 [I3 M ] [I6 P ] F ]",
        ..BASE
    },
    Level {
        name: "Stone or Lava?",
        chapter: 3,
        maps: &[&["TTTTTTTTTTTTT", "T>*.*.*.*.*PT", "TTTTTTTTTTTTT"]],
        palette: P_IFELSE,
        conds: &[Cond::StoneAhead, Cond::LavaAhead, Cond::PathAhead],
        par: 6,
        max_blocks: Some(8),
        start_blocks: 5,
        intro: Some((
            "New block: IF / ELSE",
            "Every other square is either stone OR lava - but which one? It changes every time!\n\n\
             IF / ELSE picks between two choices: IF the check is true, do the top part. \
             ELSE (otherwise), do the bottom part.",
        )),
        hint: "IF stone ahead: mine. ELSE: place a block. Then step forward twice. Repeat that 5 times.",
        lesson: "IF / ELSE means the computer always does exactly ONE of the two choices. Never both, never neither.",
        floor: Floor::Sand,
        solution: "[R5 [E3 M | P ] F F ]",
        ..BASE
    },
    Level {
        name: "Fork in the Road",
        chapter: 3,
        maps: &[
            &["TTTTTTT", "TTTTPTT", "TTTT.TT", "T>...TT", "TTTTTTT", "TTTTTTT", "TTTTTTT"],
            &["TTTTTTT", "TTTTTTT", "TTTTTTT", "T>...TT", "TTTT.TT", "TTTTPTT", "TTTTTTT"],
        ],
        palette: P_IFELSE,
        conds: TURN_CONDS,
        par: 7,
        max_blocks: Some(9),
        intro: Some((
            "Which way?",
            "At the end of this road, the path turns left OR right - you won't know which until you get there.\n\n\
             The miner can LOOK: 'path to left' is true when there's open ground on the miner's left side.",
        )),
        hint: "Walk to the end. IF path to left: turn left. ELSE: turn right. Then walk 2 more steps.",
        lesson: "The miner used its senses to choose a direction. Robots use sensors exactly like this!",
        floor: Floor::Sand,
        solution: "[R3 F ] [E1 L | R ] F F",
        ..BASE
    },
    Level {
        name: "Diamond Detector",
        chapter: 3,
        maps: &[&["TTTTTTTTTTT", "TT$$$$$$$TT", "T>.......PT", "TTTTTTTTTTT"]],
        palette: P_IFELSE,
        conds: GEM_CONDS,
        par: 7,
        max_blocks: Some(9),
        hint: "At every step: move forward, turn left to look, IF diamond ahead then mine, turn back right.",
        lesson: "Your program is a DETECTOR: it checks the world and reacts. Sensors in real robots work like this!",
        floor: Floor::Sand,
        solution: "[R7 F L [I4 M ] R ] F",
        ..BASE
    },
    Level {
        name: "Both Sides",
        chapter: 3,
        maps: &[&["TTTTTTTTTTT", "TT$$$$$$$TT", "T>.......PT", "TT$$$$$$$TT", "TTTTTTTTTTT"]],
        palette: P_IFELSE,
        conds: GEM_CONDS,
        par: 11,
        max_blocks: Some(13),
        hint: "Like Diamond Detector, but look left AND right at every step. Turn back to face forward at the end!",
        lesson: "Checking every direction carefully - that's how programs avoid missing things.",
        floor: Floor::Sand,
        solution: "[R7 F L [I4 M ] R R [I4 M ] L ] F",
        ..BASE
    },
    Level {
        name: "Mixed Bag",
        chapter: 3,
        maps: &[&["TTTTTTTTTTTTT", "T>&&&&&&&&&PT", "TTTTTTTTTTTTT"]],
        palette: P_IFELSE,
        conds: HAZARD_CONDS,
        par: 6,
        max_blocks: Some(8),
        start_blocks: 9,
        hint: "Each square could be stone, lava, or nothing. Check for stone, check for lava, then step.",
        lesson: "Your program handles three different situations. The more cases you plan for, the stronger your code!",
        floor: Floor::Sand,
        solution: "[R10 [I3 M ] [I5 P ] F ]",
        ..BASE
    },
    Level {
        name: "Rocky Corner",
        chapter: 3,
        maps: &[&["TTTTTTTT", "T>????.T", "TTTTTT?T", "TTTTTT?T", "TTTTTT?T", "TTTTTTPT", "TTTTTTTT"]],
        palette: P_IFELSE,
        conds: STONE_CONDS,
        par: 8,
        max_blocks: Some(10),
        hint: "Make a function: IF stone ahead mine, then forward. Call it in two loops, with a turn in between.",
        lesson: "Functions, loops AND decisions working together. You're writing real programs now!",
        floor: Floor::Sand,
        solution: "[R5 C ] R [R4 C ] / [I3 M ] F",
        ..BASE
    },
    Level {
        name: "Crossroads",
        chapter: 3,
        maps: &[
            &["TTTTTTT", "TTT..PT", "TTT.TTT", "T>..TTT", "TTTTTTT", "TTTTTTT", "TTTTTTT"],
            &["TTTTTTT", "TTTTTTT", "TTTTTTT", "T>..TTT", "TTT.TTT", "TTT..PT", "TTTTTTT"],
            &["TTTTTTT", "TP..TTT", "TTT.TTT", "T>..TTT", "TTTTTTT", "TTTTTTT", "TTTTTTT"],
            &["TTTTTTT", "TTTTTTT", "TTTTTTT", "T>..TTT", "TTT.TTT", "TP..TTT", "TTTTTTT"],
        ],
        palette: P_IFELSE,
        conds: TURN_CONDS,
        par: 8,
        max_blocks: Some(10),
        hint: "Walk 2, then choose a turn using IF / ELSE. That happens twice! Then walk 2 more.",
        lesson: "Four different roads, one program. Your code decides at every crossroads.",
        floor: Floor::Sand,
        solution: "[R2 F F [E1 L | R ] ] F F",
        ..BASE
    },
    Level {
        name: "Desert Storm",
        chapter: 3,
        maps: &[&["TTTTTTTTTTT", "TT$$$$$$$TT", "T>???????PT", "TTTTTTTTTTT"]],
        palette: P_IFELSE,
        conds: GEM_CONDS,
        par: 9,
        max_blocks: Some(12),
        hint: "At every step: IF stone ahead, mine. Forward. Turn left, IF diamond, mine. Turn back.",
        lesson: "Two different checks at every step - one for the path and one for treasure. Great multitasking!",
        floor: Floor::Sand,
        solution: "[R7 [I3 M ] F L [I4 M ] R ] F",
        ..BASE
    },
    Level {
        name: "Oasis Run",
        chapter: 3,
        maps: &[&[
            "TTTTTTTTT", "T>*.*.*.T", "TTTTTTT*T", "TTTTTTT.T", "TTTTTTT*T", "TTTTTTT.T", "TTTTTTT*T", "TTTTTTTPT",
            "TTTTTTTTT",
        ]],
        palette: P_IFELSE,
        conds: &[Cond::StoneAhead, Cond::LavaAhead, Cond::PathAhead],
        par: 10,
        max_blocks: Some(13),
        start_blocks: 6,
        hint: "Function: IF stone ahead mine, ELSE place. Then forward twice. Use it in two loops with a turn between.",
        lesson: "You beat the Decision Desert! IF / ELSE inside a function inside a loop - that's serious coding.",
        floor: Floor::Sand,
        solution: "[R3 C ] R [R3 C ] / [E3 M | P ] F F",
        ..BASE
    },
    // ======================= World 5: While loops & algorithms =======================
    Level {
        name: "Walk Until the Wall",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "B>...BBBBB", "BBBBPBBBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>.....BBB", "BBBBBBPBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>.......B", "BBBBBBBBPB", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: &[Cond::PathAhead, Cond::NotAtPortal],
        par: 4,
        max_blocks: Some(6),
        intro: Some((
            "New block: WHILE",
            "The hallway is a different length every time! A REPEAT needs to know the number... but you don't know it.\n\n\
             A WHILE loop keeps going AS LONG AS its check is true. \
             'While path ahead: forward' walks until the miner reaches a wall.",
        )),
        hint: "WHILE path ahead: forward. Then turn right and take one step.",
        lesson: "WHILE loops are perfect when you don't know how many times something will happen.",
        floor: Floor::Deepslate,
        solution: "[W0 F ] R F",
        ..BASE
    },
    Level {
        name: "How Far?",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBBBB", "B>....BBBBBB", "BBBBB.BBBBBB", "BBBBBPBBBBBB", "BBBBBBBBBBBB"],
            &["BBBBBBBBBBBB", "B>.......BBB", "BBBBBBBB.BBB", "BBBBBBBBPBBB", "BBBBBBBBBBBB"],
            &["BBBBBBBBBBBB", "B>.........B", "BBBBBBBBBB.B", "BBBBBBBBBBPB", "BBBBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: &[Cond::PathAhead, Cond::NotAtPortal],
        par: 5,
        max_blocks: Some(7),
        hint: "WHILE path ahead: forward. Then turn right and walk 2 more steps.",
        lesson: "Code after a WHILE loop runs once the loop is finished. Order still matters!",
        floor: Floor::Deepslate,
        solution: "[W0 F ] R F F",
        ..BASE
    },
    Level {
        name: "Stone Tunnel",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBBB", "B>###PBBBBB", "BBBBBBBBBBB"],
            &["BBBBBBBBBBB", "B>#####PBBB", "BBBBBBBBBBB"],
            &["BBBBBBBBBBB", "B>#######PB", "BBBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: WALK_CONDS,
        par: 4,
        max_blocks: Some(6),
        hint: "WHILE stone ahead: mine, forward. When the stone runs out, the portal is right there!",
        lesson: "A WHILE loop can check for anything - not just paths. It stops the moment the check becomes false.",
        floor: Floor::Deepslate,
        solution: "[W3 M F ] F",
        ..BASE
    },
    Level {
        name: "Lava River",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "BBBBPBBBBB", "BBBB.BBBBB", "B>LL.BBBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "BBBBBBPBBB", "BBBBBB.BBB", "B>LLLL.BBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "BBBBBBBBPB", "BBBBBBBB.B", "B>LLLLLL.B", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: WALK_CONDS,
        par: 7,
        max_blocks: Some(9),
        start_blocks: 6,
        hint: "WHILE lava ahead: place, forward. Then step onto the ground, turn left, and walk to the portal.",
        lesson: "You built a bridge exactly as long as it needed to be - no more, no less. Efficient!",
        floor: Floor::Deepslate,
        solution: "[W5 P F ] F L F F",
        ..BASE
    },
    Level {
        name: "Right Turns Only",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "B>....BBBB", "BBBBB.BBBB", "BBBBB.BBBB", "BP....BBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>.......B", "BBBBBBBB.B", "BBBBBBBB.B", "BBBBBBBB.B", "BBBP.....B", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>......BB", "BBBBBBB.BB", "BP......BB", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: PATH_CONDS,
        par: 4,
        max_blocks: Some(6),
        intro: Some((
            "Follow the road",
            "This road only ever turns RIGHT, but the lengths change every time.\n\n\
             Put an IF / ELSE inside a WHILE: while you're not at the portal, IF there's a path ahead go forward, ELSE turn right.",
        )),
        hint: "WHILE not at portal: IF path ahead { forward } ELSE { turn right }",
        lesson: "A tiny rule, repeated, can follow any road. That's the magic of algorithms!",
        floor: Floor::Deepslate,
        solution: "[W7 [E0 F | R ] ]",
        ..BASE
    },
    Level {
        name: "Stony Spiral",
        chapter: 4,
        maps: &[&[
            "BBBBBBBBB", "B>.?.?..B", "BBBBBBB.B", "B..?..B.B", "B.BBB.B?B", "B?BP..B.B", "B.BBBBB.B", "B.?.?...B",
            "BBBBBBBBB",
        ]],
        palette: P_ALL,
        conds: WALK_CONDS,
        par: 6,
        max_blocks: Some(8),
        hint: "Same as Right Turns Only, but first check: IF stone ahead, mine it.",
        lesson: "You added one new rule to an old algorithm. Improving code step by step is how real software grows.",
        floor: Floor::Deepslate,
        solution: "[W7 [I3 M ] [E0 F | R ] ]",
        ..BASE
    },
    Level {
        name: "Winding Road",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "B>...BBBBB", "BBBB.BBBBB", "BBBB....BB", "BBBBBBB.BB", "BBP.....BB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "BBBB...PBB", "BBBB.BBBBB", "BB...BBBBB", "BB.BBBBBBB", "BB^BBBBBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>......BB", "BBBBBBB.BB", "BBBBBBB.BB", "BBBP....BB", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: PATH_CONDS,
        par: 6,
        max_blocks: Some(9),
        intro: Some((
            "Left AND right",
            "Now the road turns both ways! Your program needs to LOOK and DECIDE at every step.\n\n\
             You can put an IF / ELSE inside another IF / ELSE to choose between three things!",
        )),
        hint: "WHILE not at portal: IF path ahead, go forward. ELSE, IF path to left, turn left, ELSE turn right.",
        lesson: "You combined loops and decisions. That's how self-driving robots follow a road!",
        floor: Floor::Deepslate,
        solution: "[W7 [E0 F | [E1 L | R ] ] ]",
        ..BASE
    },
    Level {
        name: "Rocky Road",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "B>.?.BBBBB", "BBBB.BBBBB", "BBBB..?.BB", "BBBBBBB.BB", "BBP?.?..BB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>.?.?..BB", "BBBBBBB.BB", "BBBBBBB?BB", "BBBP.?..BB", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 8,
        max_blocks: Some(11),
        hint: "Start with your Winding Road program. Add 'IF stone ahead: mine' at the top of the WHILE loop.",
        lesson: "Each part of your program has one job: one part clears the way, the other part steers.",
        floor: Floor::Deepslate,
        solution: "[W7 [I3 M ] [E0 F | [E1 L | R ] ] ]",
        ..BASE
    },
    Level {
        name: "Lava Road",
        chapter: 4,
        maps: &[
            &["BBBBBBBBBB", "B>.!.BBBBB", "BBBB.BBBBB", "BBBB..!.BB", "BBBBBBB.BB", "BBP!.!..BB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "BBBB..!PBB", "BBBB.BBBBB", "BB...BBBBB", "BB!BBBBBBB", "BB^BBBBBBB", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 8,
        max_blocks: Some(11),
        start_blocks: 4,
        hint: "Like Rocky Road, but check for lava and place a block instead of mining.",
        lesson: "Same algorithm, different danger. Once you know a pattern, you can adapt it to new problems fast!",
        floor: Floor::Deepslate,
        solution: "[W7 [I5 P ] [E0 F | [E1 L | R ] ] ]",
        ..BASE
    },
    Level {
        name: "The Maze",
        chapter: 4,
        maps: &[
            &[
                "BBBBBBBBBBB", "B>..B.....B", "BBB.B.BBB.B", "B...B.B...B", "B.B.BBB.B.B",
                "B.B.....B.B", "B.B.B.BBB.B", "B...B....PB", "BBBBBBBBBBB",
            ],
            &[
                "BBBBBBBBBBB", "B>B.......B", "B.BBBBB.B.B", "B.....B.B.B", "B.BBB.B.B.B",
                "B...B.B.B.B", "B.B.B.BBB.B", "B.B......PB", "BBBBBBBBBBB",
            ],
        ],
        palette: P_ALL,
        conds: PATH_CONDS,
        par: 7,
        max_blocks: Some(10),
        intro: Some((
            "The secret of mazes",
            "Here's a trick explorers use: keep your RIGHT hand touching the wall, and you'll always find the way out!\n\n\
             That means: if you can turn right, do it. If not, go straight. If you can't go straight, turn left.\n\n\
             A step-by-step recipe that always works is called an ALGORITHM.",
        )),
        hint: "WHILE not at portal: IF path to right { turn right, forward } ELSE { IF path ahead { forward } ELSE { turn left } }",
        lesson: "You programmed an ALGORITHM! The 'right-hand rule' solves mazes - and computers use algorithms for everything.",
        floor: Floor::Deepslate,
        solution: "[W7 [E2 R F | [E0 F | L ] ] ]",
        ..BASE
    },
    Level {
        name: "Bigger Maze",
        chapter: 4,
        maps: &[
            &[
                "BBBBBBBBBBBBB", "B>..B.....B.B", "BBB.BBB.B.B.B", "B...B...B.B.B", "B.BBB.BBB.B.B", "B.......B...B",
                "B.B.BBB.BBB.B", "B.B.....B.B.B", "B.BBB.BBB.B.B", "B.....B....PB", "BBBBBBBBBBBBB",
            ],
            &[
                "BBBBBBBBBBBBB", "B>B.......B.B", "B.B.BBBBB.B.B", "B.B...B.B...B", "B.B.B.B.BBB.B", "B.B.B.....B.B",
                "B.B.B.B.B.B.B", "B.B.B.B.B...B", "B.BBB.B.BBBBB", "B.....B....PB", "BBBBBBBBBBBBB",
            ],
        ],
        palette: P_ALL,
        conds: PATH_CONDS,
        par: 7,
        max_blocks: Some(10),
        hint: "Your maze algorithm from The Maze works on ANY maze like this - even a giant one. Try it!",
        lesson: "A good algorithm doesn't care how big the problem is. That's why programmers love them!",
        floor: Floor::Deepslate,
        solution: "[W7 [E2 R F | [E0 F | L ] ] ]",
        ..BASE
    },
    Level {
        name: "Deep Dark Gauntlet",
        chapter: 4,
        maps: &[&[
            "BBBBBBBBBBBB", "B>.?.!.?..BB", "BBBBBBBBB.BB", "BBBBBBBBB?BB", "BBBBBBBBB.BB",
            "BB..!.?...BB", "BB.BBBBBBBBB", "BBPBBBBBBBBB", "BBBBBBBBBBBB",
        ]],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 10,
        max_blocks: Some(14),
        start_blocks: 3,
        hint: "Inside a WHILE loop: IF stone ahead, mine. IF lava ahead, place. Then follow the road like in Winding Road.",
        lesson: "You conquered the Deep Dark! Loops, decisions and algorithms all working together.",
        floor: Floor::Deepslate,
        solution: "[W7 [I3 M ] [I5 P ] [E0 F | [E1 L | R ] ] ]",
        ..BASE
    },
    // ======================= World 6: Master challenges =======================
    Level {
        name: "Lava Sea",
        chapter: 5,
        maps: &[
            &["BBBBBBBBBB", "B>&&&.BBBB", "BBBBB&BBBB", "BBBBB&BBBB", "BP&&&.BBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>&.&.&..B", "BBBBBBBB&B", "BBBBBBBB.B", "BBBBBBBB&B", "BBBP&.&..B", "BBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 8,
        max_blocks: Some(11),
        start_blocks: 8,
        intro: Some((
            "Welcome to the Nether Fortress",
            "These are MASTER challenges. Every level mixes everything you've learned, and every level changes each time.\n\n\
             Real programmers solve big problems by building a little, testing, and building a little more. \
             Use STEP to watch your code closely. You've got this!",
        )),
        hint: "WHILE not at portal: IF stone, mine. IF lava, place. IF path ahead, forward, ELSE turn right.",
        lesson: "Three checks and a turn rule, inside one loop. You're thinking like a real programmer!",
        floor: Floor::Nether,
        solution: "[W7 [I3 M ] [I5 P ] [E0 F | R ] ]",
        ..BASE
    },
    Level {
        name: "Lava Bridges",
        chapter: 5,
        maps: &[&[
            "BBBBBBBBBBB", "B>LL!.LL!.B", "BBBBBBBBBLB", "BBBBBBBBBLB", "BBBBBBBBB!B", "BBBBBBBBB.B",
            "BBBBBBBBBPB", "BBBBBBBBBBB",
        ]],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 9,
        max_blocks: Some(12),
        start_blocks: 9,
        hint: "Function: WHILE lava ahead { place, forward }, then WHILE path ahead { forward }. Call it, call it, turn right, call it!",
        lesson: "WHILE loops inside a function: one small tool that handles bridges of ANY length.",
        floor: Floor::Nether,
        solution: "C C R C / [W5 P F ] [W0 F ]",
        ..BASE
    },
    Level {
        name: "Rubble Maze",
        chapter: 5,
        maps: &[&[
            "BBBBBBBBBBB", "B>..B..?..B", "BBB.B.BBB.B", "B...B.B...B", "B.B.BBB.B.B",
            "B?B..?..B.B", "B.B.B.BBB.B", "B...B....PB", "BBBBBBBBBBB",
        ]],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 9,
        max_blocks: Some(12),
        hint: "Use your maze algorithm, but when you'd turn left, first check: IF stone ahead, mine it instead!",
        lesson: "You upgraded a famous algorithm to handle a new problem. That's what computer scientists do!",
        floor: Floor::Nether,
        solution: "[W7 [E2 R F | [E0 F | [E3 M | L ] ] ] ]",
        ..BASE
    },
    Level {
        name: "Diamond Patrol",
        chapter: 5,
        maps: &[
            &["BBBBBBBBBB", "BB$$$$BBBB", "B>.....BBB", "BB$$$$.BBB", "BBBBBBPBBB", "BBBBBBBBBB"],
            &["BBBBBBBBBBBB", "BB$$$$$$BBBB", "B>.......BBB", "BB$$$$$$.BBB", "BBBBBBBBPBBB", "BBBBBBBBBBBB"],
        ],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 14,
        max_blocks: Some(17),
        hint: "Function: look left (mine if diamond), look right (mine if diamond), face forward. \
               Main: WHILE path ahead { forward, call }. Then head down to the portal.",
        lesson: "A WHILE loop that calls a function full of IFs. That's how real programs are structured!",
        floor: Floor::Nether,
        solution: "[W0 F C ] R F F / L [I4 M ] R R [I4 M ] L",
        ..BASE
    },
    Level {
        name: "Nether Maze",
        chapter: 5,
        maps: &[&[
            "BBBBBBBBBBB", "B>B.......B", "B!BBBBB.B.B", "B.....B.B.B", "B.BBB.B.B.B",
            "B..!B.B.B.B", "B.B.B.BBB.B", "B.B...!..PB", "BBBBBBBBBBB",
        ]],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 9,
        max_blocks: Some(12),
        start_blocks: 4,
        hint: "Before your maze algorithm makes a choice, check: IF lava ahead, place a block.",
        lesson: "Your algorithm stays calm even when the maze is full of danger. Robust code handles surprises!",
        floor: Floor::Nether,
        solution: "[W7 [I5 P ] [E2 R F | [E0 F | L ] ] ]",
        ..BASE
    },
    Level {
        name: "Master Miner",
        chapter: 5,
        maps: &[
            &["BBBBBBBBBB", "B>.&.BBBBB", "BBBB.BBBBB", "BBBB..&.BB", "BBBBBBB.BB", "BBP&.&..BB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "B>.&.&..BB", "BBBBBBB.BB", "BBBBBBB&BB", "BBBP.&..BB", "BBBBBBBBBB"],
            &["BBBBBBBBBB", "BBBB..&PBB", "BBBB.BBBBB", "BB...BBBBB", "BB&BBBBBBB", "BB^BBBBBBB", "BBBBBBBBBB"],
            &[
                "BBBBBBBBBBBB", "B>.&.&.&..BB", "BBBBBBBBB.BB", "BBBBBBBBB&BB", "BBBBBBBBB.BB",
                "BB..&.&...BB", "BB.BBBBBBBBB", "BBPBBBBBBBBB", "BBBBBBBBBBBB",
            ],
        ],
        palette: P_ALL,
        conds: ALL_CONDS,
        par: 10,
        max_blocks: Some(14),
        start_blocks: 6,
        intro: Some((
            "The Final Challenge",
            "This is it - the last level! The road could be ANY shape, and any square might hold stone or lava.\n\n\
             Everything you've learned is in here somewhere. Take your time, test often, and use STEP to find bugs.",
        )),
        hint: "WHILE not at portal: IF stone, mine. IF lava, place. Then steer: IF path ahead forward, ELSE IF path to left turn left, ELSE turn right.",
        lesson: "You're a MASTER CODER! Sequences, loops, functions, decisions and algorithms are the same ideas behind every app and game in the world.",
        floor: Floor::Nether,
        solution: "[W7 [I3 M ] [I5 P ] [E0 F | [E1 L | R ] ] ]",
        ..BASE
    },
];
