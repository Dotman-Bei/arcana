import React from "react";
import { PlayingCard } from "./PlayingCard";
import { parseCard } from "@/lib/poker";

type Props = {
  community: number[];   // raw card values 0-51 (5 elements)
  communityFlags: number; // bitmask: bits 0-2=flop, 3=turn, 4=river
  state: number;
};

export const CommunityCards: React.FC<Props> = ({ community, communityFlags, state }) => {
  const showFlop  = (communityFlags & 0b00000111) !== 0;
  const showTurn  = (communityFlags & 0b00001000) !== 0;
  const showRiver = (communityFlags & 0b00010000) !== 0;

  return (
    <div className="flex flex-col items-center gap-3">
      <span className="text-xs text-white/40 uppercase tracking-widest">Community Cards</span>
      <div className="flex gap-3 items-center">
        {[0, 1, 2].map(i => (
          <PlayingCard
            key={i}
            card={showFlop && community[i] > 0 ? parseCard(community[i]) : undefined}
            faceDown={!showFlop || community[i] === 0}
          />
        ))}
        <div className="w-px h-16 bg-white/10 mx-1" />
        <PlayingCard
          card={showTurn && community[3] > 0 ? parseCard(community[3]) : undefined}
          faceDown={!showTurn || community[3] === 0}
        />
        <PlayingCard
          card={showRiver && community[4] > 0 ? parseCard(community[4]) : undefined}
          faceDown={!showRiver || community[4] === 0}
        />
      </div>
    </div>
  );
};
