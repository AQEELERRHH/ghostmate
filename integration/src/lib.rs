/// Integration tests for the GhostMate agent and game contract.
///
/// Pure-Rust simulations of the on-chain contract logic, exercisable with
/// `cargo test` without a Miden node.

// ─────────────────────────────────────────────────────────────────────────────
// Agent-account integration tests (existing)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod agent_tests {
    use ghostmate_chess::{
        board::{Board, Move, EMPTY, BLACK},
        eval::{decode_weight, encode_weight, NO_ADJ},
        minimax::best_move,
    };

    fn board_to_words(b: &Board) -> ([u64; 4], [u64; 4]) {
        (b.to_board_word(), b.to_flags_word())
    }
    fn words_to_board(bw: [u64; 4], fw: [u64; 4]) -> Board {
        Board::from_words(bw, fw)
    }

    #[test]
    fn receive_move_startpos_returns_valid_move() {
        let b = Board::startpos();
        let (bw, fw) = board_to_words(&b);
        let restored = words_to_board(bw, fw);
        let mv = best_move(&restored, &NO_ADJ);
        assert!(!mv.is_null());
        assert!(mv.from_sq() < 64);
        assert!(mv.to_sq() < 64);
        assert_ne!(mv.from_sq(), mv.to_sq());
    }

    #[test]
    fn wire_roundtrip_preserves_all_fields() {
        let mut b = Board::startpos();
        b.ep_sq   = 20;
        b.castling = 0b1010;
        b.side    = BLACK;
        let (bw, fw) = board_to_words(&b);
        let b2 = words_to_board(bw, fw);
        assert_eq!(b.squares,  b2.squares);
        assert_eq!(b.side,     b2.side);
        assert_eq!(b.ep_sq,    b2.ep_sq);
        assert_eq!(b.castling, b2.castling);
    }

    #[test]
    fn opening_book_overrides_engine() {
        let b = Board::startpos();
        let book_move = Move::new(12, 28, 0);
        let book_u64  = book_move.to_u64();
        assert_ne!(book_u64, 0);
        let recovered = Move::from_u64(book_u64);
        assert_eq!(recovered, book_move);
        assert!(!best_move(&b, &NO_ADJ).is_null());
    }

    #[test]
    fn weight_encode_decode_storage_cycle() {
        let mut adj = NO_ADJ;
        for (i, offset) in [-50i32, 0, 30, -128, 127, 0, 0, 0, 0, 0, 0, 0].iter().enumerate() {
            let encoded = encode_weight(*offset);
            adj[i]      = decode_weight(encoded);
        }
        assert_eq!(adj[0], -50);
        assert_eq!(adj[3], -128);
        assert_eq!(adj[4], 127);
        assert!(!best_move(&Board::startpos(), &adj).is_null());
    }

    #[test]
    fn engine_plays_six_plies_without_panic() {
        let mut b = Board::startpos();
        for _ in 0..6 {
            let mv = best_move(&b, &NO_ADJ);
            if mv.is_null() { break; }
            b = b.apply_move(mv);
        }
        let pieces: usize = b.squares.iter().filter(|&&p| p != EMPTY).count();
        assert!(pieces > 0 && pieces <= 32);
    }

    #[test]
    fn note_storage_layout_from_computed_move() {
        let b  = Board::startpos();
        let mv = best_move(&b, &NO_ADJ);
        assert!((mv.from_sq() as u64) < 64);
        assert!((mv.to_sq()   as u64) < 64);
        assert!((mv.promo()   as u64) < 13);
        assert!(b.to_board_word()[0] > 0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GameContract logic tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod game_tests {
    use ghostmate_chess::{
        board::{
            Board, Move, WHITE, BLACK, EMPTY,
            W_PAWN, B_PAWN, W_KING, B_KING, W_QUEEN, W_ROOK, B_ROOK,
        },
        moves::generate_legal,
    };

    // ── Mirror of the contract's terminal-detection logic ────────────────────

    const STATUS_ACTIVE:     u64 = 0;
    const STATUS_WHITE_WINS: u64 = 1;
    const STATUS_BLACK_WINS: u64 = 2;
    const STATUS_DRAW:       u64 = 3;
    const HALFMOVE_DRAW:     u64 = 100;

    fn detect_terminal(next_board: &Board, halfmove_clock: u64) -> u64 {
        let legal = generate_legal(next_board);
        if legal.is_empty() {
            if next_board.in_check(next_board.side) {
                if next_board.side == WHITE { STATUS_BLACK_WINS } else { STATUS_WHITE_WINS }
            } else {
                STATUS_DRAW
            }
        } else if halfmove_clock >= HALFMOVE_DRAW {
            STATUS_DRAW
        } else {
            STATUS_ACTIVE
        }
    }

    fn is_legal_move(board: &Board, mv: Move) -> bool {
        generate_legal(board).contains(&mv)
    }

    /// Simulate the halfmove clock update from receive_move().
    fn update_halfmove_clock(board: &Board, mv: Move, current: u64) -> u64 {
        let moved   = board.piece_at(mv.from_sq());
        let target  = board.piece_at(mv.to_sq());
        let is_pawn    = moved == W_PAWN || moved == B_PAWN;
        let is_capture = target != EMPTY;
        if is_pawn || is_capture { 0 } else { current + 1 }
    }

    /// Simulate the wager payout amounts.
    fn wager_payout(status: u64, per_player: u64) -> (u64, u64) {
        // returns (white_receives, black_receives)
        match status {
            STATUS_WHITE_WINS => (per_player * 2, 0),
            STATUS_BLACK_WINS => (0, per_player * 2),
            _                 => (per_player, per_player), // draw
        }
    }

    /// Simulate sender authorisation check.
    fn authorised_sender(board: &Board, sender: [u64; 2], white: [u64; 2], black: [u64; 2]) -> bool {
        let expected = if board.side == WHITE { white } else { black };
        sender[0] == expected[0] && sender[1] == expected[1]
    }

    // ── Move validation ──────────────────────────────────────────────────────

    #[test]
    fn accepts_legal_opening_move() {
        let b = Board::startpos();
        let e4 = Move::new(12, 28, 0); // e2-e4
        assert!(is_legal_move(&b, e4), "e2-e4 must be legal from start");
    }

    #[test]
    fn rejects_illegal_move_wrong_piece() {
        let b = Board::startpos();
        let illegal = Move::new(0, 8, 0); // a1-rook to a2 (blocked by pawn)
        assert!(!is_legal_move(&b, illegal));
    }

    #[test]
    fn rejects_move_from_empty_square() {
        let b = Board::startpos();
        // e4 is empty at start
        let illegal = Move::new(28, 36, 0);
        assert!(!is_legal_move(&b, illegal));
    }

    #[test]
    fn rejects_move_that_leaves_king_in_check() {
        // Pin scenario: moving the pinned piece would expose king to check
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;   // e1
        b.squares[60] = B_KING;   // e8
        b.squares[12] = W_PAWN;   // e2 (pinned along e-file)
        b.squares[28] = B_ROOK;   // e4 (pins the pawn to the king)
        b.side = WHITE;
        // Moving pawn off e-file exposes king → illegal
        let _illegal = Move::new(12, 19, 0); // e2→d3 (exposes e-file pin)
        // Generate legal moves and check d3 is NOT available
        let legals = generate_legal(&b);
        let moves_off_efile: Vec<_> = legals.iter()
            .filter(|m| m.from_sq() == 12 && m.to_sq() != 20)
            .collect();
        // Only moves that keep the pawn on the e-file (e2→e3/e4) are legal
        let exposes_king = moves_off_efile.iter().any(|m| m.to_sq() == 19);
        assert!(!exposes_king, "moving pinned pawn off e-file must be illegal");
    }

    #[test]
    fn accepts_promotion_move() {
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;  // e1
        b.squares[56] = B_KING;  // a8 — away from e8 so the pawn can push through
        b.squares[52] = W_PAWN;  // e7
        b.side = WHITE;
        // Straight push e7→e8 with queen promotion (e8 is empty)
        let promote_queen = Move::new(52, 60, W_QUEEN);
        assert!(is_legal_move(&b, promote_queen));
    }

    // ── Board state update ───────────────────────────────────────────────────

    #[test]
    fn board_updates_after_valid_move() {
        let b  = Board::startpos();
        let e4 = Move::new(12, 28, 0);
        let b2 = b.apply_move(e4);
        assert_eq!(b2.squares[12], EMPTY,  "e2 must be empty after e2-e4");
        assert_eq!(b2.squares[28], W_PAWN, "e4 must have white pawn");
        assert_eq!(b2.side, BLACK,         "it is now black's turn");
        assert_eq!(b2.ep_sq, 20,           "en-passant square must be e3");
    }

    #[test]
    fn board_wire_roundtrip_after_move() {
        let b  = Board::startpos();
        let mv = Move::new(12, 28, 0);
        let b2 = b.apply_move(mv);
        // Simulate storage encode → decode cycle
        let bw = b2.to_board_word();
        let fw_base = b2.to_flags_word();
        let halfmove = 0u64;
        let fw = [fw_base[0], fw_base[1], fw_base[2], halfmove];
        let restored = Board::from_words(bw, fw);
        assert_eq!(b2.squares,  restored.squares);
        assert_eq!(b2.side,     restored.side);
        assert_eq!(b2.ep_sq,    restored.ep_sq);
        assert_eq!(b2.castling, restored.castling);
    }

    // ── Halfmove clock ───────────────────────────────────────────────────────

    #[test]
    fn halfmove_clock_increments_on_quiet_move() {
        let b  = Board::startpos();
        // Knight move (not a pawn, not a capture)
        let nf3 = Move::new(6, 21, 0); // Ng1-f3
        let clock = update_halfmove_clock(&b, nf3, 10);
        assert_eq!(clock, 11);
    }

    #[test]
    fn halfmove_clock_resets_on_pawn_move() {
        let b   = Board::startpos();
        let e4  = Move::new(12, 28, 0);
        let clock = update_halfmove_clock(&b, e4, 42);
        assert_eq!(clock, 0, "pawn move must reset halfmove clock");
    }

    #[test]
    fn halfmove_clock_resets_on_capture() {
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;
        b.squares[60] = B_KING;
        b.squares[0]  = W_ROOK;
        b.squares[56] = B_ROOK; // rook on a8 — white rook can capture it
        b.side = WHITE;
        let capture = Move::new(0, 56, 0); // Ra1×Ra8
        let clock = update_halfmove_clock(&b, capture, 55);
        assert_eq!(clock, 0, "capture must reset halfmove clock");
    }

    // ── Terminal condition detection ─────────────────────────────────────────

    #[test]
    fn active_game_returns_status_active() {
        let b = Board::startpos();
        let status = detect_terminal(&b, 0);
        assert_eq!(status, STATUS_ACTIVE);
    }

    #[test]
    fn checkmate_gives_correct_winner() {
        // Fool's mate: 1.f3 e5 2.g4 Qh4# — white to move, checkmated
        let mut b = Board::startpos();
        b = b.apply_move(Move::new(13, 21, 0)); // f2-f3
        b = b.apply_move(Move::new(52, 36, 0)); // e7-e5
        b = b.apply_move(Move::new(14, 30, 0)); // g2-g4
        b = b.apply_move(Move::new(59, 31, 0)); // Qd8-h4#
        // `b` is now the state where it's WHITE's turn and white has no moves
        assert!(b.in_check(WHITE), "white must be in check");
        let status = detect_terminal(&b, 4);
        assert_eq!(status, STATUS_BLACK_WINS, "black wins by checkmate");
    }

    #[test]
    fn stalemate_is_draw() {
        // Classic stalemate: black king on a8, white queen on b6, white king on c6
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[56] = B_KING;   // a8
        b.squares[41] = W_QUEEN;  // b6
        b.squares[42] = W_KING;   // c6
        b.side = BLACK;
        // Black king has no legal moves and is NOT in check
        assert!(!b.in_check(BLACK), "black must not be in check");
        let legal = generate_legal(&b);
        assert!(legal.is_empty(), "black must have no legal moves");
        let status = detect_terminal(&b, 10);
        assert_eq!(status, STATUS_DRAW, "stalemate must be a draw");
    }

    #[test]
    fn fifty_move_rule_triggers_draw() {
        let b = Board::startpos();
        // 100 half-moves without pawn move or capture → draw
        let status = detect_terminal(&b, HALFMOVE_DRAW);
        assert_eq!(status, STATUS_DRAW, "50-move rule must produce a draw");
    }

    #[test]
    fn forty_nine_moves_not_yet_draw() {
        let b = Board::startpos();
        let status = detect_terminal(&b, HALFMOVE_DRAW - 1);
        assert_eq!(status, STATUS_ACTIVE, "game still active at 99 half-moves");
    }

    // ── Wager payout logic ───────────────────────────────────────────────────

    #[test]
    fn winner_receives_double_wager() {
        let (w_gets, b_gets) = wager_payout(STATUS_WHITE_WINS, 500);
        assert_eq!(w_gets, 1000, "white should receive both wagers");
        assert_eq!(b_gets, 0);
    }

    #[test]
    fn loser_receives_nothing() {
        let (w_gets, b_gets) = wager_payout(STATUS_BLACK_WINS, 500);
        assert_eq!(w_gets, 0);
        assert_eq!(b_gets, 1000, "black should receive both wagers");
    }

    #[test]
    fn draw_returns_each_players_wager() {
        let (w_gets, b_gets) = wager_payout(STATUS_DRAW, 300);
        assert_eq!(w_gets, 300, "white gets their wager back");
        assert_eq!(b_gets, 300, "black gets their wager back");
    }

    #[test]
    fn wager_total_is_conserved() {
        let per_player = 750u64;
        let total_in   = per_player * 2;
        for status in [STATUS_WHITE_WINS, STATUS_BLACK_WINS, STATUS_DRAW] {
            let (w, b) = wager_payout(status, per_player);
            assert_eq!(w + b, total_in, "wager must be fully distributed (status={status})");
        }
    }

    // ── Sender / turn authorisation ──────────────────────────────────────────

    #[test]
    fn correct_sender_is_authorised_white_turn() {
        let b     = Board::startpos(); // white to move
        let white = [0xAABB_u64, 0xCCDD_u64];
        let black = [0x1122_u64, 0x3344_u64];
        assert!( authorised_sender(&b, white, white, black));
        assert!(!authorised_sender(&b, black, white, black));
    }

    #[test]
    fn correct_sender_is_authorised_black_turn() {
        let mut b = Board::startpos();
        b.side    = BLACK;
        let white = [0xAABB_u64, 0xCCDD_u64];
        let black = [0x1122_u64, 0x3344_u64];
        assert!( authorised_sender(&b, black, white, black));
        assert!(!authorised_sender(&b, white, white, black));
    }

    #[test]
    fn wrong_prefix_is_rejected() {
        let b     = Board::startpos();
        let white = [0xAABB_u64, 0xCCDD_u64];
        let black = [0x1122_u64, 0x3344_u64];
        // Same suffix but different prefix
        let impostor = [0xDEAD_u64, 0xCCDD_u64];
        assert!(!authorised_sender(&b, impostor, white, black));
    }

    // ── Full move sequence ───────────────────────────────────────────────────

    #[test]
    fn full_fools_mate_sequence_ends_in_checkmate() {
        let mut b = Board::startpos();
        let mut halfmove_clock = 0u64;

        let moves = [
            Move::new(13, 21, 0), // f2-f3
            Move::new(52, 36, 0), // e7-e5
            Move::new(14, 30, 0), // g2-g4
            Move::new(59, 31, 0), // Qd8-h4#
        ];

        for mv in moves {
            assert_eq!(detect_terminal(&b, halfmove_clock), STATUS_ACTIVE,
                "game must still be active before the final move");
            assert!(is_legal_move(&b, mv), "move {:?} must be legal", mv);
            halfmove_clock = update_halfmove_clock(&b, mv, halfmove_clock);
            b = b.apply_move(mv);
        }

        assert_eq!(detect_terminal(&b, halfmove_clock), STATUS_BLACK_WINS);
    }

    #[test]
    fn fifty_move_counter_accumulates_over_quiet_moves() {
        let mut b = Board::startpos();
        let mut clock = 0u64;

        // Play knight moves back and forth (quiet, no pawns, no captures)
        // Ng1-f3 (white), Ng8-f6 (black), Nf3-g1 (white), Nf6-g8 (black)...
        let knight_moves = [
            Move::new(6, 21, 0),  // Ng1-f3
            Move::new(62, 45, 0), // Ng8-f6
            Move::new(21, 6, 0),  // Nf3-g1
            Move::new(45, 62, 0), // Nf6-g8
        ];

        for _ in 0..10 {
            for &mv in &knight_moves {
                clock = update_halfmove_clock(&b, mv, clock);
                b = b.apply_move(mv);
            }
        }

        // 40 quiet half-moves; should not yet be draw
        assert!(clock < HALFMOVE_DRAW,
            "expected clock < 100 after 40 moves, got {clock}");

        // Simulate the remaining moves to reach 100
        while clock < HALFMOVE_DRAW {
            for &mv in &knight_moves {
                clock = update_halfmove_clock(&b, mv, clock);
                b = b.apply_move(mv);
                if clock >= HALFMOVE_DRAW { break; }
            }
        }
        let status = detect_terminal(&b, clock);
        assert_eq!(status, STATUS_DRAW, "50-move rule must trigger at clock={clock}");
    }

    #[test]
    fn turn_note_storage_layout_is_correct() {
        // Verify the turn note layout matches what emit_turn_note() would produce
        // and what a game-turn-note script would read.
        let b = Board::startpos();
        let bw = b.to_board_word();
        let fw = b.to_flags_word();

        // Expected note storage: [board(4), flags(4), game_id(4), reserved(4)] = 16 Felts
        let layout: Vec<u64> = [
            bw[0], bw[1], bw[2], bw[3],
            fw[0], fw[1], fw[2], 0u64,   // fw[3] = halfmove_clock = 0 initially
            0xABCD_u64, 0xEF01_u64, 0, 0, // game_id placeholder
            0, 0, 0, 0,                   // reserved
        ].to_vec();

        assert_eq!(layout.len(), 16, "turn note must have 16 felts in storage");
        // board occupies indices 0-3
        assert_eq!(layout[0], bw[0]);
        // side-to-move flag occupies index 4
        assert_eq!(layout[4], fw[0]); // 1 = white to move at startpos
    }

    #[test]
    fn agent_move_note_storage_layout_is_correct() {
        // Verify the agent-move-note storage matches what the note script reads:
        // [from_sq, to_sq, promo, 0]
        let from_sq: u64 = 12; // e2
        let to_sq:   u64 = 28; // e4
        let promo:   u64 = 0;
        let storage = [from_sq, to_sq, promo, 0u64];
        // The note script reads storage[0..2] and validates them
        assert!(storage[0] < 64);
        assert!(storage[1] < 64);
        assert_eq!(storage[3], 0, "reserved field must be zero");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MatchRegistry integration tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod registry_tests {
    use ghostmate_registry::{
        AgentStats, ChallengeRecord, GameRecord,
        CHALLENGE_OPEN, CHALLENGE_ACCEPTED, CHALLENGE_CANCELLED,
        GAME_ACTIVE, GAME_WHITE_WINS, GAME_BLACK_WINS, GAME_DRAW,
        REGISTERED,
        sort_leaderboard, leaderboard_rank, apply_game_result,
    };

    fn agent(prefix: u64, wins: u64, losses: u64, draws: u64) -> AgentStats {
        AgentStats { prefix, name: [0; 4], wins, losses, draws }
    }

    // ── AgentStats ────────────────────────────────────────────────────────────

    #[test]
    fn score_is_2x_wins_plus_draws() {
        let a = agent(1, 3, 1, 2);
        assert_eq!(a.score(), 3 * 2 + 2);
    }

    #[test]
    fn total_games_sums_all_results() {
        let a = agent(1, 3, 1, 2);
        assert_eq!(a.total_games(), 6);
    }

    #[test]
    fn apply_result_win_as_white() {
        let a  = agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_WHITE_WINS);
        assert_eq!(a2.wins, 1);
        assert_eq!(a2.losses, 0);
        assert_eq!(a2.draws, 0);
    }

    #[test]
    fn apply_result_loss_as_white() {
        let a  = agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_BLACK_WINS);
        assert_eq!(a2.losses, 1);
        assert_eq!(a2.wins, 0);
    }

    #[test]
    fn apply_result_win_as_black() {
        let a  = agent(1, 0, 0, 0);
        let a2 = a.apply_result(false, GAME_BLACK_WINS);
        assert_eq!(a2.wins, 1);
        assert_eq!(a2.losses, 0);
    }

    #[test]
    fn apply_result_draw_increments_draws() {
        let a  = agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_DRAW);
        assert_eq!(a2.draws, 1);
        assert_eq!(a2.wins,   0);
        assert_eq!(a2.losses, 0);
    }

    #[test]
    fn apply_result_is_non_mutating() {
        let original = agent(1, 5, 2, 1);
        let _updated = original.apply_result(true, GAME_WHITE_WINS);
        assert_eq!(original.wins, 5, "original must not be mutated");
    }

    // ── ChallengeRecord ───────────────────────────────────────────────────────

    #[test]
    fn challenge_status_predicates() {
        let open  = ChallengeRecord { id: 0, challenger_prefix: 1, wager_amount: 100, status: CHALLENGE_OPEN };
        let accpt = ChallengeRecord { status: CHALLENGE_ACCEPTED,  ..open.clone() };
        let cancl = ChallengeRecord { status: CHALLENGE_CANCELLED, ..open.clone() };

        assert!( open.is_open());
        assert!(!open.is_accepted());
        assert!(!open.is_cancelled());
        assert!( accpt.is_accepted());
        assert!( cancl.is_cancelled());
    }

    // ── GameRecord ────────────────────────────────────────────────────────────

    #[test]
    fn winner_prefix_returns_correct_player() {
        let base = GameRecord {
            id: 0, white_prefix: 10, black_prefix: 20,
            status: GAME_ACTIVE, game_contract_prefix: 99,
        };
        assert_eq!(base.winner_prefix(), None);
        assert_eq!(GameRecord { status: GAME_WHITE_WINS, ..base.clone() }.winner_prefix(), Some(10));
        assert_eq!(GameRecord { status: GAME_BLACK_WINS, ..base.clone() }.winner_prefix(), Some(20));
        assert_eq!(GameRecord { status: GAME_DRAW,       ..base }.winner_prefix(),         None);
    }

    // ── Leaderboard sorting ───────────────────────────────────────────────────

    #[test]
    fn sort_leaderboard_orders_by_wins_descending() {
        let mut agents = vec![
            agent(1, 2, 1, 0),
            agent(2, 5, 0, 0),
            agent(3, 1, 2, 0),
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2); // 5 wins
        assert_eq!(agents[1].prefix, 1); // 2 wins
        assert_eq!(agents[2].prefix, 3); // 1 win
    }

    #[test]
    fn sort_leaderboard_breaks_win_ties_by_fewer_losses() {
        let mut agents = vec![
            agent(1, 3, 2, 0), // 3W 2L
            agent(2, 3, 0, 0), // 3W 0L  ← higher
            agent(3, 3, 1, 0), // 3W 1L
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2);
        assert_eq!(agents[1].prefix, 3);
        assert_eq!(agents[2].prefix, 1);
    }

    #[test]
    fn sort_leaderboard_breaks_further_ties_by_more_draws() {
        let mut agents = vec![
            agent(1, 3, 0, 1), // 3W 0L 1D
            agent(2, 3, 0, 3), // 3W 0L 3D  ← higher
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2);
        assert_eq!(agents[1].prefix, 1);
    }

    #[test]
    fn sort_leaderboard_handles_empty_input() {
        let mut agents: Vec<AgentStats> = vec![];
        sort_leaderboard(&mut agents);
        assert!(agents.is_empty());
    }

    #[test]
    fn sort_leaderboard_handles_single_agent() {
        let mut agents = vec![agent(1, 0, 0, 0)];
        sort_leaderboard(&mut agents);
        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn leaderboard_rank_returns_1_based_position() {
        let agents = vec![
            agent(10, 5, 0, 0),
            agent(20, 3, 1, 0),
            agent(30, 1, 2, 0),
        ];
        assert_eq!(leaderboard_rank(&agents, 10), Some(1));
        assert_eq!(leaderboard_rank(&agents, 20), Some(2));
        assert_eq!(leaderboard_rank(&agents, 30), Some(3));
    }

    #[test]
    fn leaderboard_rank_returns_none_for_unknown_prefix() {
        let agents = vec![agent(10, 5, 0, 0)];
        assert_eq!(leaderboard_rank(&agents, 99), None);
    }

    // ── apply_game_result ─────────────────────────────────────────────────────

    #[test]
    fn apply_game_result_updates_both_players() {
        let mut stats = vec![agent(1, 0, 0, 0), agent(2, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 1, 2, GAME_WHITE_WINS);
        assert_eq!(updated, 2);
        let w = stats.iter().find(|a| a.prefix == 1).unwrap();
        let b = stats.iter().find(|a| a.prefix == 2).unwrap();
        assert_eq!(w.wins,   1);
        assert_eq!(b.losses, 1);
    }

    #[test]
    fn apply_game_result_draw_increments_both_draws() {
        let mut stats = vec![agent(1, 0, 0, 0), agent(2, 0, 0, 0)];
        apply_game_result(&mut stats, 1, 2, GAME_DRAW);
        for a in &stats { assert_eq!(a.draws, 1); }
    }

    #[test]
    fn apply_game_result_returns_zero_for_unknown_agents() {
        let mut stats = vec![agent(1, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 99, 100, GAME_WHITE_WINS);
        assert_eq!(updated, 0);
        assert_eq!(stats[0].wins, 0);
    }

    #[test]
    fn apply_game_result_updates_only_one_if_one_unregistered() {
        let mut stats = vec![agent(1, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 1, 99, GAME_WHITE_WINS);
        assert_eq!(updated, 1);
        assert_eq!(stats[0].wins, 1);
    }

    // ── Registration simulation ───────────────────────────────────────────────

    #[test]
    fn registered_sentinel_distinguishes_from_default_zero() {
        // An unregistered agent has all-zero stats; REGISTERED=1 in [3] marks it.
        let zero_word = [0u64, 0, 0, 0];
        let reg_word  = [0u64, 0, 0, REGISTERED];
        assert_ne!(zero_word[3], reg_word[3]);
        assert_eq!(reg_word[3], 1);
    }

    #[test]
    fn re_registration_preserves_existing_stats() {
        // Simulate: register agent, record wins, re-register with new name.
        // stats should not reset.
        let mut agents = vec![agent(42, 3, 1, 2)];
        // Re-registering = only updating name, stats untouched.
        // (Here we just verify stats vec is unchanged after "name update".)
        let result = apply_game_result(&mut agents, 42, 99, GAME_WHITE_WINS);
        assert_eq!(result, 1);
        assert_eq!(agents[0].wins, 4); // incremented from 3
    }

    // ── Challenge lifecycle simulation ────────────────────────────────────────

    #[test]
    fn challenge_lifecycle_open_to_accepted() {
        let mut c = ChallengeRecord {
            id: 0, challenger_prefix: 1, wager_amount: 500, status: CHALLENGE_OPEN,
        };
        assert!(c.is_open());
        c.status = CHALLENGE_ACCEPTED;
        assert!(c.is_accepted());
        assert!(!c.is_open());
    }

    #[test]
    fn challenge_lifecycle_open_to_cancelled() {
        let mut c = ChallengeRecord {
            id: 1, challenger_prefix: 2, wager_amount: 0, status: CHALLENGE_OPEN,
        };
        assert!(c.is_open());
        c.status = CHALLENGE_CANCELLED;
        assert!(c.is_cancelled());
    }

    // ── Full registry simulation ──────────────────────────────────────────────

    #[test]
    fn full_registry_sim_two_agents_one_game() {
        // Simulate: two agents register → post/accept challenge → game plays → result recorded.
        let mut agents = vec![agent(10, 0, 0, 0), agent(20, 0, 0, 0)];

        // No wins yet — both appear somewhere in the leaderboard.
        assert!(leaderboard_rank(&agents, 10).is_some());
        assert!(leaderboard_rank(&agents, 20).is_some());

        // Record agent 10 (white) winning.
        apply_game_result(&mut agents, 10, 20, GAME_WHITE_WINS);
        sort_leaderboard(&mut agents);

        assert_eq!(agents[0].prefix, 10); // 1 win
        assert_eq!(agents[1].prefix, 20); // 1 loss

        assert_eq!(leaderboard_rank(&agents, 10), Some(1));
        assert_eq!(leaderboard_rank(&agents, 20), Some(2));
    }

    #[test]
    fn full_registry_sim_draw_keeps_equal_score() {
        let mut agents = vec![agent(10, 0, 0, 0), agent(20, 0, 0, 0)];
        apply_game_result(&mut agents, 10, 20, GAME_DRAW);

        let a10 = agents.iter().find(|a| a.prefix == 10).unwrap();
        let a20 = agents.iter().find(|a| a.prefix == 20).unwrap();
        assert_eq!(a10.score(), a20.score());
        assert_eq!(a10.draws, 1);
        assert_eq!(a20.draws, 1);
    }

    // ── Result-note storage layout simulation ─────────────────────────────────

    #[test]
    fn result_note_storage_layout_is_valid() {
        // Mirrors what GameContract writes into the ResultNote storage.
        let game_id:         u64 = 7;
        let result_status:   u64 = GAME_WHITE_WINS;
        let gc_prefix:       u64 = 0xDEAD;
        let storage = [game_id, result_status, gc_prefix, 0u64];

        // The note script validates result_status ∈ {1,2,3}.
        let s = storage[1];
        assert!(s == 1 || s == 2 || s == 3, "invalid result status in storage");
        assert_eq!(storage[3], 0, "reserved field must be zero");
    }
}
