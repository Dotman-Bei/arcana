import React, { useEffect, useRef } from "react";

export type LogEntry = {
  id: number;
  ts: number;
  type: "deal" | "action" | "arcium" | "info" | "error" | "settle";
  message: string;
};

const TYPE_STYLES: Record<LogEntry["type"], string> = {
  deal:   "text-blue-400",
  action: "text-white/80",
  arcium: "text-purple-400",
  info:   "text-white/50",
  error:  "text-red-400",
  settle: "text-gold-400",
};

const TYPE_PREFIXES: Record<LogEntry["type"], string> = {
  deal:   "🃏",
  action: "►",
  arcium: "🔒",
  info:   "·",
  error:  "✗",
  settle: "🏆",
};

type Props = {
  entries: LogEntry[];
};

export const GameLog: React.FC<Props> = ({ entries }) => {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries.length]);

  return (
    <div className="flex flex-col gap-0.5 h-48 overflow-y-auto bg-black/30 rounded-lg p-3 font-mono text-xs">
      {entries.length === 0 && (
        <span className="text-white/30 italic">Game log will appear here…</span>
      )}
      {entries.map(entry => (
        <div key={entry.id} className={`flex gap-2 ${TYPE_STYLES[entry.type]}`}>
          <span className="opacity-40 flex-shrink-0">
            {new Date(entry.ts).toLocaleTimeString("en", { hour12: false })}
          </span>
          <span className="flex-shrink-0">{TYPE_PREFIXES[entry.type]}</span>
          <span>{entry.message}</span>
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
};
