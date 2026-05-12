/**
 * Poker card utilities for Arcana Hold'em.
 *
 * Card encoding (0-51):
 *   rank = card % 13  →  0=A, 1=2, 2=3, ..., 9=10, 10=J, 11=Q, 12=K
 *   suit = card / 13  →  0=♣, 1=♦, 2=♥, 3=♠
 */

export const RANK_LABELS = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
export const SUIT_LABELS = ["♣", "♦", "♥", "♠"];
export const SUIT_NAMES  = ["clubs", "diamonds", "hearts", "spades"];

export type Card = {
  value: number;  // 0-51
  rank: number;   // 0-12
  suit: number;   // 0-3
  rankLabel: string;
  suitLabel: string;
  isRed: boolean;
};

export const parseCard = (value: number): Card => {
  const rank = value % 13;
  const suit = Math.floor(value / 13);
  return {
    value,
    rank,
    suit,
    rankLabel: RANK_LABELS[rank],
    suitLabel: SUIT_LABELS[suit],
    isRed: suit === 1 || suit === 2,
  };
};

export const cardLabel = (card: Card) => `${card.rankLabel}${card.suitLabel}`;

/** Unmask a hole card: card_raw = (masked_value - mask) wrapping mod 2^64 */
export const unmaskCard = (maskedValue: bigint, mask: bigint): number => {
  const MASK64 = 0xffff_ffff_ffff_ffffn;
  return Number((maskedValue - mask + (1n << 64n)) & MASK64) & 0xff;
};

// ── Hand name display ─────────────────────────────────────────────────────────

export const HAND_NAMES = [
  "High Card",
  "One Pair",
  "Two Pair",
  "Three of a Kind",
  "Straight",
  "Flush",
  "Full House",
  "Four of a Kind",
  "Straight Flush",
];

export const handNameFromScore = (score: bigint): string => {
  const category = Number(score / 10_000_000_000n);
  return HAND_NAMES[Math.min(category, HAND_NAMES.length - 1)];
};

// ── Game state helpers ────────────────────────────────────────────────────────

export const STATE_LABELS: Record<number, string> = {
  0: "Waiting for Player",
  1: "Dealing...",
  2: "Pre-Flop",
  3: "Flop",
  4: "Turn",
  5: "River",
  6: "Showdown",
  7: "Settled",
};

export const lamportsToSol = (lamports: number | bigint): string =>
  (Number(lamports) / 1e9).toFixed(4);
