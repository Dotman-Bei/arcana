import React from "react";
import type { Card } from "@/lib/poker";

type Props = {
  card?: Card;
  faceDown?: boolean;
  size?: "sm" | "md";
  className?: string;
};

export const PlayingCard: React.FC<Props> = ({ card, faceDown = false, size = "md", className = "" }) => {
  const sizeClass = size === "sm" ? "w-12 h-18" : "w-16 h-24";

  if (faceDown || !card) {
    return (
      <div
        className={`card-back ${sizeClass} ${className}`}
        title="Hidden card"
      />
    );
  }

  return (
    <div className={`card-face ${card.isRed ? "red" : "black"} ${sizeClass} ${className}`}>
      <span className="text-xs font-bold leading-none">{card.rankLabel}</span>
      <span className="text-base leading-none text-center self-center">{card.suitLabel}</span>
      <span className="text-xs font-bold leading-none self-end rotate-180">{card.rankLabel}</span>
    </div>
  );
};
