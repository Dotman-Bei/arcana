import React, { useState, useEffect, useCallback, useRef } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { PublicKey } from "@solana/web3.js";
import { Toaster, toast } from "sonner";
import { PlayerSeat } from "@/components/PlayerSeat";
import { CommunityCards } from "@/components/CommunityCards";
import { BettingPanel } from "@/components/BettingPanel";
import { GameLog, type LogEntry } from "@/components/GameLog";
import {
  fetchTable,
  initTable,
  joinTable,
  submitAction,
  revealShowdown,
  type TableAccount,
} from "@/lib/holdem-client";
import { parseCard, STATE_LABELS, lamportsToSol, unmaskCard } from "@/lib/poker";
import { deriveTablePda } from "@/lib/arcium-config";
import { isConfigured } from "@/lib/arcium-config";
import { solanaConnection } from "@/lib/solana";

// Lazy-load the IDL after build.
// In development, replace with: import IDL from "../../target/idl/arcana_holdem.json";
const IDL = {} as object; // placeholder — replace with actual IDL after anchor build

const BUY_IN_SOL = 0.01;
const BIG_BLIND_SOL = 0.001;
const BUY_IN  = BigInt(Math.round(BUY_IN_SOL  * 1e9));
const BIG_BLIND = BigInt(Math.round(BIG_BLIND_SOL * 1e9));

let logCounter = 0;
const makeLog = (type: LogEntry["type"], message: string): LogEntry => ({
  id: ++logCounter, ts: Date.now(), type, message,
});

export const App: React.FC = () => {
  const wallet = useWallet();
  const [log, setLog] = useState<LogEntry[]>([]);
  const [table, setTable] = useState<TableAccount | null>(null);
  const [tablePda, setTablePda] = useState<PublicKey | null>(null);
  // Persist mask across refreshes so reveal_showdown still works after a page reload.
const [myMask, setMyMask] = useState<bigint | null>(() => {
  try {
    const stored = localStorage.getItem("arcana_my_mask");
    return stored ? BigInt(stored) : null;
  } catch { return null; }
});
  const [joinTableId, setJoinTableId] = useState("");
  const [busy, setBusy] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const addLog = useCallback((type: LogEntry["type"], message: string) => {
    setLog(prev => [...prev, makeLog(type, message)]);
  }, []);

  // Poll table state every 3 seconds when a game is active.
  useEffect(() => {
    if (!tablePda) return;
    pollRef.current = setInterval(async () => {
      const t = await fetchTable(tablePda, IDL).catch(() => null);
      if (t) setTable(t);
    }, 3000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [tablePda]);

  const isPlayerA = table && wallet.publicKey && table.playerA.equals(wallet.publicKey);
  const isPlayerB = table && wallet.publicKey && table.playerB.equals(wallet.publicKey);
  const myState = isPlayerA ? 0 : isPlayerB ? 1 : -1;
  const myStack = isPlayerA ? (table?.playerAStack ?? 0n) : (table?.playerBStack ?? 0n);
  const myBet   = isPlayerA ? (table?.playerABet ?? 0n)   : (table?.playerBBet ?? 0n);
  const oppBet  = isPlayerA ? (table?.playerBBet ?? 0n)   : (table?.playerABet ?? 0n);
  const myTurn  = table && table.currentTurn === myState;

  const myCard1 = table && myMask != null
    ? (isPlayerA ? unmaskCard(table.playerACard1, myMask) : unmaskCard(table.playerBCard1, myMask))
    : null;
  const myCard2 = table && myMask != null
    ? (isPlayerA ? unmaskCard(table.playerACard2, myMask) : unmaskCard(table.playerBCard2, myMask))
    : null;

  const handleCreateTable = async () => {
    if (!wallet.connected || !wallet.publicKey) { toast.error("Connect wallet first"); return; }
    setBusy(true);
    try {
      addLog("info", "Creating table on Solana devnet…");
      const { tablePda: pda, mask, sig } = await initTable(wallet, IDL, BUY_IN, BIG_BLIND);
      setTablePda(pda);
      setMyMask(mask);
      localStorage.setItem("arcana_my_mask", mask.toString());
      localStorage.setItem("arcana_player_a_mask", mask.toString()); // shared with Player B for MVP demo
      localStorage.setItem("arcana_table_pda", pda.toBase58());
      const t = await fetchTable(pda, IDL);
      setTable(t);
      addLog("info", `Table created: ${pda.toBase58()}`);
      addLog("arcium", "Waiting for player B to join and trigger Arcium deal…");
      toast.success(`Table created! Share table ID: ${pda.toBase58()}`);
    } catch (e: any) {
      addLog("error", e.message);
      toast.error(e.message);
    } finally { setBusy(false); }
  };

  const handleJoinTable = async () => {
    if (!wallet.connected || !wallet.publicKey) { toast.error("Connect wallet first"); return; }
    if (!joinTableId.trim()) { toast.error("Enter a table ID"); return; }
    setBusy(true);
    try {
      const pda = new PublicKey(joinTableId.trim());
      addLog("info", `Joining table ${pda.toBase58()}…`);
      addLog("arcium", "Encrypting deck seed + masks for Arcium MXE…");

      // Player A must share their mask out-of-band (lobby link, QR, etc.).
      // For the MVP demo, we check localStorage first (same browser / same session).
      const storedMaskA = localStorage.getItem("arcana_player_a_mask");
      if (!storedMaskA) {
        throw new Error("Player A's mask not found. Ask Player A to share their mask before joining.");
      }
      const playerAMask = BigInt(storedMaskA);

      const { mask, sig } = await joinTable(wallet, IDL, pda, playerAMask);
      setTablePda(pda);
      setMyMask(mask);
      localStorage.setItem("arcana_my_mask", mask.toString());
      localStorage.setItem("arcana_table_pda", pda.toBase58());
      const t = await fetchTable(pda, IDL);
      setTable(t);
      addLog("arcium", `Arcium deal_cards computation queued (tx: ${sig.slice(0, 8)}…)`);
      addLog("arcium", "MXE shuffling deck privately… waiting for callback (~10s)");
      toast.success("Joined table! Waiting for Arcium to deal cards…");
    } catch (e: any) {
      addLog("error", e.message);
      toast.error(e.message);
    } finally { setBusy(false); }
  };

  const handleAction = async (action: "Fold" | "Check" | "Call" | "Raise", raiseTo = 0n) => {
    if (!tablePda || !table || !wallet.publicKey) return;
    setBusy(true);
    try {
      const winnerWallet = action === "Fold" ? (isPlayerA ? table.playerB : table.playerA) : wallet.publicKey!;
      addLog("action", `You ${action.toLowerCase()}${action === "Raise" ? ` to ${lamportsToSol(raiseTo)} SOL` : ""}`);
      const sig = await submitAction(wallet, IDL, tablePda, action, raiseTo, winnerWallet);
      addLog("info", `tx: ${sig.slice(0, 16)}…`);
      const t = await fetchTable(tablePda, IDL);
      setTable(t);
    } catch (e: any) {
      addLog("error", e.message);
      toast.error(e.message);
    } finally { setBusy(false); }
  };

  const handleReveal = async () => {
    if (!tablePda || !table || !wallet.publicKey || myMask == null) return;
    setBusy(true);
    try {
      addLog("arcium", "Revealing hole-card mask for on-chain hand evaluation…");
      const sig = await revealShowdown(wallet, IDL, tablePda, myMask, table.playerA, table.playerB);
      addLog("settle", `Mask revealed (tx: ${sig.slice(0, 16)}…)`);
      const t = await fetchTable(tablePda, IDL);
      setTable(t);
      if (t?.state === 7) addLog("settle", "Game settled!");
    } catch (e: any) {
      addLog("error", e.message);
      toast.error(e.message);
    } finally { setBusy(false); }
  };

  const gameState = table?.state ?? -1;
  const stateLabel = STATE_LABELS[gameState] ?? "Unknown";
  const configured = isConfigured();

  return (
    <div className="min-h-screen flex flex-col items-center px-4 py-8 gap-6">
      <Toaster richColors position="top-right" />

      {/* Header */}
      <div className="flex items-center justify-between w-full max-w-4xl">
        <div>
          <h1 className="text-3xl font-bold text-gold-400 tracking-tight">Arcana Hold'em</h1>
          <p className="text-sm text-white/50">Private on-chain poker — powered by Arcium MXE</p>
        </div>
        <div className="flex flex-col items-end gap-1">
          <WalletMultiButton />
          {!configured && (
            <span className="text-xs text-yellow-500">
              ⚠ Set VITE_ARCIUM_* env vars to enable on-chain play
            </span>
          )}
        </div>
      </div>

      {/* Lobby */}
      {!tablePda && (
        <div className="w-full max-w-md bg-white/5 rounded-2xl p-6 flex flex-col gap-4">
          <h2 className="text-lg font-semibold text-white">Start a Game</h2>
          <p className="text-sm text-white/50">
            Buy-in: <strong className="text-white">{BUY_IN_SOL} SOL</strong> &nbsp;|&nbsp;
            Big Blind: <strong className="text-white">{BIG_BLIND_SOL} SOL</strong>
          </p>

          <button
            onClick={handleCreateTable}
            disabled={busy || !wallet.connected}
            className="w-full py-3 bg-gold-500 hover:bg-gold-400 disabled:opacity-40 text-black font-bold rounded-xl transition-colors"
          >
            {busy ? "Creating…" : "Create Table (Player A)"}
          </button>

          <div className="flex items-center gap-2 text-white/20">
            <div className="flex-1 h-px bg-white/10" />
            <span className="text-xs">or</span>
            <div className="flex-1 h-px bg-white/10" />
          </div>

          <div className="flex gap-2">
            <input
              value={joinTableId}
              onChange={e => setJoinTableId(e.target.value)}
              placeholder="Paste table ID…"
              className="flex-1 px-3 py-2 text-sm bg-white/10 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:border-gold-400"
            />
            <button
              onClick={handleJoinTable}
              disabled={busy || !wallet.connected || !joinTableId.trim()}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-40 text-white font-semibold rounded-lg transition-colors"
            >
              Join
            </button>
          </div>
        </div>
      )}

      {/* Poker Table */}
      {tablePda && table && (
        <div className="w-full max-w-4xl flex flex-col gap-4">
          {/* State badge */}
          <div className="flex items-center justify-between">
            <span className="text-sm text-white/60">
              Table: <span className="font-mono text-white/80">{tablePda.toBase58().slice(0, 20)}…</span>
            </span>
            <span className="px-3 py-1 bg-white/10 rounded-full text-sm font-semibold text-gold-400">
              {stateLabel}
            </span>
          </div>

          {/* Table felt */}
          <div className="felt-table rounded-3xl p-8 flex flex-col items-center gap-8">
            {/* Opponent seat */}
            <PlayerSeat
              label="Player B"
              address={table.playerB.equals(PublicKey.default) ? undefined : table.playerB.toBase58()}
              stack={table.playerBStack}
              bet={table.playerBBet}
              isHidden={isPlayerA}
              isActive={table.currentTurn === 1}
              isDealer={false}
            />

            {/* Community cards */}
            <CommunityCards
              community={table.community}
              communityFlags={table.communityFlags}
              state={table.state}
            />

            {/* My seat */}
            <PlayerSeat
              label="Player A"
              address={table.playerA.toBase58()}
              stack={table.playerAStack}
              bet={table.playerABet}
              card1={myCard1 !== null && isPlayerA ? parseCard(myCard1) : undefined}
              card2={myCard2 !== null && isPlayerA ? parseCard(myCard2) : undefined}
              isHidden={!isPlayerA}
              isActive={table.currentTurn === 0}
              isDealer
            />
          </div>

          {/* Betting panel */}
          {(myState === 0 || myState === 1) && (
            <BettingPanel
              pot={table.pot}
              myBet={myBet}
              oppBet={oppBet}
              myStack={myStack}
              bigBlind={table.bigBlind}
              canAct={!!myTurn && !busy}
              isShowdown={gameState === 6}
              onFold={() => handleAction("Fold")}
              onCheck={() => handleAction("Check")}
              onCall={() => handleAction("Call")}
              onRaise={amt => handleAction("Raise", amt)}
              onReveal={handleReveal}
            />
          )}

          {/* Privacy proof callout */}
          <div className="bg-purple-900/30 border border-purple-500/20 rounded-xl p-4 text-sm">
            <p className="text-purple-300 font-semibold mb-1">🔒 Arcium MXE Privacy</p>
            <p className="text-purple-200/70">
              Your hole cards are masked with a secret XOR key before being written on-chain.
              The Arcium Multi-party eXecution Environment shuffled the deck in a garbled circuit —
              neither player, the chain, nor any node ever saw the plaintext cards during the deal.
            </p>
          </div>
        </div>
      )}

      {/* Game log */}
      {log.length > 0 && (
        <div className="w-full max-w-4xl">
          <h3 className="text-xs text-white/40 uppercase tracking-widest mb-2">Event Log</h3>
          <GameLog entries={log} />
        </div>
      )}
    </div>
  );
};
