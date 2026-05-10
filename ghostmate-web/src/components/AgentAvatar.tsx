import { useMemo } from 'react';
import { hashStr } from '../utils/agentUtils';

interface Props {
  name: string;
  size?: number;
}

export function AgentAvatar({ name, size = 56 }: Props) {
  const { pixels, colors } = useMemo(() => {
    const seed = hashStr(name);
    const ROWS = 8, HALF = 4;

    // deterministic hash at position
    const at = (i: number) => ((seed ^ (i * 2654435761)) >>> 0) & 1;

    const pixels: boolean[][] = Array.from({ length: ROWS }, (_, r) =>
      Array.from({ length: HALF }, (_, c) => at(r * HALF + c) === 1),
    );
    // Force eyes at row 2, col 1
    pixels[2][1] = true;
    // Force chin outline at row 6
    pixels[6][0] = true;
    // Antenna at row 0 col 2
    pixels[0][2] = true;

    const hue  = seed % 360;
    const hue2 = (seed * 7 + 120) % 360;
    return {
      pixels,
      colors: {
        bg:      `hsl(${hue},40%,8%)`,
        primary: `hsl(${hue},75%,60%)`,
        eye:     `hsl(${hue2},90%,70%)`,
      },
    };
  }, [name]);

  const ROWS = 8, HALF = 4;
  const fullCols = 7;
  const cell = size / fullCols;
  const w = fullCols * cell;
  const h = ROWS * cell;

  const rects: React.ReactElement[] = [];
  for (let r = 0; r < ROWS; r++) {
    for (let hc = 0; hc < HALF; hc++) {
      if (!pixels[r][hc]) continue;
      const isEye = r === 2 && hc === 1;
      const fill  = isEye ? colors.eye : colors.primary;
      // left side
      rects.push(
        <rect key={`l${r}${hc}`} x={hc * cell} y={r * cell} width={cell} height={cell} fill={fill} />,
      );
      // mirror (skip center column)
      const mirrorCol = fullCols - 1 - hc;
      if (mirrorCol !== hc) {
        rects.push(
          <rect key={`r${r}${hc}`} x={mirrorCol * cell} y={r * cell} width={cell} height={cell} fill={isEye ? colors.eye : colors.primary} />,
        );
      }
    }
  }

  return (
    <svg
      width={w}
      height={h}
      viewBox={`0 0 ${w} ${h}`}
      style={{ imageRendering: 'pixelated', borderRadius: 4 }}
    >
      <rect width={w} height={h} fill={colors.bg} />
      {rects}
    </svg>
  );
}
