/** Pure Unicode-aware subtitle segmentation for CJK and Latin text. */

const SENTENCE_ENDINGS = new Set(["。", "！", "？", "!", "?", "；", ";", "\n"]);
const PREFERRED_BREAKS = new Set([
  "，",
  "、",
  ",",
  "：",
  ":",
  "—",
  "–",
  "-",
  " ",
]);

/** Splits `text` into subtitle segments of at most `maximumCharacters`. */
export function segments(text: string, maximumCharacters: number): string[] {
  const max = Math.max(4, maximumCharacters);
  const remaining = Array.from(text.trim());
  const result: string[] = [];

  while (remaining.length > 0) {
    trimLeadingWhitespace(remaining);
    if (remaining.length === 0) break;

    const searchCount = Math.min(max, remaining.length);
    const sentenceEndIndex = remaining
      .slice(0, searchCount)
      .findIndex((character) => SENTENCE_ENDINGS.has(character));

    if (sentenceEndIndex >= 0) {
      appendSegment(remaining, sentenceEndIndex + 1, result);
      continue;
    }

    if (remaining.length <= max) {
      appendSegment(remaining, remaining.length, result);
      continue;
    }

    const minimumPreferredBreak = Math.max(1, Math.floor(max / 2));
    let preferredBreak = -1;
    for (let index = max - 1; index >= minimumPreferredBreak; index -= 1) {
      if (PREFERRED_BREAKS.has(remaining[index])) {
        preferredBreak = index;
        break;
      }
    }

    let end: number;
    if (preferredBreak >= 0) {
      // A whitespace break character is dropped (not swallowed into a segment).
      end = isWhitespace(remaining[preferredBreak])
        ? preferredBreak
        : preferredBreak + 1;
    } else {
      end = max;
    }
    appendSegment(remaining, Math.max(1, end), result);
  }

  return result;
}

/** Returns only the trailing `maximumSegments` segments of a running draft. */
export function visibleDraftSegments(
  text: string,
  maximumCharacters: number,
  maximumSegments = 2,
): string[] {
  const clamped = Math.max(0, maximumSegments);
  if (clamped === 0) return [];
  return segments(text, maximumCharacters).slice(-clamped);
}

function appendSegment(
  remaining: string[],
  end: number,
  result: string[],
): void {
  const safeEnd = Math.min(Math.max(1, end), remaining.length);
  const segment = remaining.slice(0, safeEnd).join("").trim();
  remaining.splice(0, safeEnd);
  if (segment.length > 0) {
    result.push(segment);
  }
}

function trimLeadingWhitespace(characters: string[]): void {
  while (characters.length > 0 && isWhitespace(characters[0])) {
    characters.shift();
  }
}

function isWhitespace(character: string): boolean {
  return /\s/.test(character);
}
