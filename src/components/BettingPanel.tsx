import React, { useState } from "react";
import { lamportsToSol } from "@/lib/poker";

type Props = {
  pot: bigint;
  myBet: bigint;
  oppBet: bigint;
  myStack: bigint;
  bigBlind: bigint;
  canAct: boolean;
  isShowdown: boolean;
  onFold: () => void;
  onCheck: () => void;
  onCall: () => void;
  onRaise: (amount: bigint) => void;
  onReveal: () => void;
};

export const BettingPanel: React.FC<Props> = ({
  pot,
  myBet,
  oppBet,
  myStack,
  bigBlind,
  canAct,
  isShowdown,
  onFold,
  onCheck,
  onCall,
  onRaise,
  onReveal,
}) => {
  const [raiseAmount, setRaiseAmount] = useState<string>("");

  const callAmount = oppBet > myBet ? oppBet - myBet : 0n;
  const canCheck = myBet >= oppBet;
  const minRaise = oppBet + bigBlind;

  if (isShowdown) {
    return (
      <div className="flex flex-col items-center gap-4 p-4 bg-white/5 rounded-xl">
        <p className="text-sm text-white/60 text-center">
          Showdown — reveal your hole-card mask to settle the pot.
        </p>
        <button
          onClick={onReveal}
          disabled={!canAct}
          className="px-6 py-2 bg-gold-500 hover:bg-gold-400 disabled:opacity-40 text-black font-bold rounded-lg transition-colors"
        >
          Reveal Hand
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 p-4 bg-white/5 rounded-xl">
      {/* Pot */}
      <div className="text-center">
        <span className="text-xs text-white/40 uppercase tracking-widest">Pot</span>
        <p className="text-xl font-bold text-gold-400">{lamportsToSol(pot)} SOL</p>
      </div>

      {/* Action buttons */}
      <div className="flex gap-2 flex-wrap justify-center">
        <button
          onClick={onFold}
          disabled={!canAct}
          className="px-4 py-2 bg-red-800 hover:bg-red-700 disabled:opacity-40 text-white text-sm font-semibold rounded-lg transition-colors"
        >
          Fold
        </button>

        {canCheck ? (
          <button
            onClick={onCheck}
            disabled={!canAct}
            className="px-4 py-2 bg-blue-700 hover:bg-blue-600 disabled:opacity-40 text-white text-sm font-semibold rounded-lg transition-colors"
          >
            Check
          </button>
        ) : (
          <button
            onClick={onCall}
            disabled={!canAct || myStack < callAmount}
            className="px-4 py-2 bg-green-700 hover:bg-green-600 disabled:opacity-40 text-white text-sm font-semibold rounded-lg transition-colors"
          >
            Call {lamportsToSol(callAmount)} SOL
          </button>
        )}

        <div className="flex gap-1.5 items-center">
          <input
            type="number"
            value={raiseAmount}
            onChange={e => setRaiseAmount(e.target.value)}
            placeholder={`≥ ${lamportsToSol(minRaise)}`}
            disabled={!canAct}
            className="w-28 px-2 py-1 text-sm bg-white/10 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:border-gold-400"
          />
          <button
            onClick={() => {
              const parsed = parseFloat(raiseAmount);
              if (!isNaN(parsed)) onRaise(BigInt(Math.round(parsed * 1e9)));
            }}
            disabled={!canAct || !raiseAmount}
            className="px-3 py-2 bg-yellow-700 hover:bg-yellow-600 disabled:opacity-40 text-white text-sm font-semibold rounded-lg transition-colors"
          >
            Raise
          </button>
        </div>
      </div>
    </div>
  );
};
