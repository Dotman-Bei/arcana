import React from "react";
import { PlayingCard } from "./PlayingCard";
import type { Card } from "@/lib/poker";
import { lamportsToSol } from "@/lib/poker";

type Props = {
  label: string;
  address?: string;
  stack: bigint;
  bet: bigint;
  card1?: Card;
  card2?: Card;
  isHidden: boolean;   // true when cards belong to opponent (face down)
  isActive: boolean;   // true when it's this player's turn
  isDealer: boolean;
};

export const PlayerSeat: React.FC<Props> = ({
  label,
  address,
  stack,
  bet,
  card1,
  card2,
  isHidden,
  isActive,
  isDealer,
}) => {
  const truncAddr = address ? `${address.slice(0, 4)}…${address.slice(-4)}` : "Empty seat";

  return (
    <div
      className={`flex flex-col items-center gap-2 p-4 rounded-xl transition-all duration-300
        ${isActive ? "bg-gold-400/10 ring-2 ring-gold-400" : "bg-white/5"}`}
    >
      {/* Player info */}
      <div className="flex items-center gap-2">
        {isDealer && (
          <span className="text-xs bg-gold-500 text-black font-bold rounded-full px-2 py-0.5">D</span>
        )}
        <span className="text-sm font-semibold text-white/90">{label}</span>
        {isActive && (
          <span className="w-2 h-2 rounded-full bg-gold-400 chip-pulse" />
        )}
      </div>

      <span className="text-xs text-white/50 font-mono">{truncAddr}</span>

      {/* Hole cards */}
      <div className="flex gap-2">
        <PlayingCard card={isHidden ? undefined : card1} faceDown={isHidden || !card1} />
        <PlayingCard card={isHidden ? undefined : card2} faceDown={isHidden || !card2} />
      </div>

      {/* Stack and current bet */}
      <div className="flex flex-col items-center gap-0.5">
        <span className="text-sm text-green-400 font-mono font-bold">
          {lamportsToSol(stack)} SOL
        </span>
        {bet > 0n && (
          <span className="text-xs text-yellow-400 font-mono">
            Bet: {lamportsToSol(bet)} SOL
          </span>
        )}
      </div>
    </div>
  );
};
