import { useImperativeHandle, useState } from "react";
import type { Ref } from "react";
import type { Color, Size } from "../types";
import { VoteOption } from "../VoteOption/VoteOption";

export interface VoteSectionHandle {
  getSelected: () => string[];
}

export interface VoteSectionProps {
  size: Size;
  color: Color;
  options: string[];
  className?: string;
  ref?: Ref<VoteSectionHandle>;
}

export function VoteSection({
  size,
  color,
  options,
  className = "",
  ref,
}: VoteSectionProps) {
  const [selected, setSelected] = useState<string[]>([]);

  useImperativeHandle(
    ref,
    () => ({
      getSelected: () => selected,
    }),
    [selected],
  );

  function toggle(option: string) {
    setSelected((prev) =>
      prev.includes(option)
        ? prev.filter((o) => o !== option)
        : [...prev, option],
    );
  }

  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      {options.map((option) => (
        <VoteOption
          key={option}
          size={size}
          color={color}
          label={option}
          selected={selected.includes(option)}
          onClick={() => toggle(option)}
        />
      ))}
    </div>
  );
}
