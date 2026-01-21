import { useMemo, useState } from 'react';

interface DiffLine {
  type: 'unchanged' | 'added' | 'removed' | 'header';
  originalLineNum?: number;
  currentLineNum?: number;
  content: string;
}

interface DiffViewerProps {
  originalContent: string;
  currentContent: string;
  filePath: string;
}

/**
 * Simple line-by-line diff algorithm
 * For production, consider using a library like 'diff' for better results
 */
function computeDiff(original: string, current: string): DiffLine[] {
  const originalLines = original.split('\n');
  const currentLines = current.split('\n');

  // LCS-based diff using dynamic programming
  const m = originalLines.length;
  const n = currentLines.length;

  // Build LCS table
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (originalLines[i - 1] === currentLines[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to find diff
  let i = m;
  let j = n;
  const result: DiffLine[] = [];

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && originalLines[i - 1] === currentLines[j - 1]) {
      result.unshift({
        type: 'unchanged',
        originalLineNum: i,
        currentLineNum: j,
        content: originalLines[i - 1],
      });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      result.unshift({
        type: 'added',
        currentLineNum: j,
        content: currentLines[j - 1],
      });
      j--;
    } else {
      result.unshift({
        type: 'removed',
        originalLineNum: i,
        content: originalLines[i - 1],
      });
      i--;
    }
  }

  return result;
}

export function DiffViewer({ originalContent, currentContent, filePath }: DiffViewerProps) {
  const [viewMode, setViewMode] = useState<'unified' | 'split'>('unified');

  const diffLines = useMemo(
    () => computeDiff(originalContent, currentContent),
    [originalContent, currentContent]
  );

  const stats = useMemo(() => {
    const added = diffLines.filter(l => l.type === 'added').length;
    const removed = diffLines.filter(l => l.type === 'removed').length;
    return { added, removed };
  }, [diffLines]);

  return (
    <div className="diff-viewer">
      <div className="diff-viewer__header">
        <div className="diff-viewer__file-info">
          <span className="diff-viewer__file-name">{filePath}</span>
          <span className="diff-viewer__stats">
            <span className="diff-viewer__stat diff-viewer__stat--added">+{stats.added}</span>
            <span className="diff-viewer__stat diff-viewer__stat--removed">-{stats.removed}</span>
          </span>
        </div>
        <div className="diff-viewer__controls">
          <button
            className={`diff-viewer__mode-btn ${viewMode === 'unified' ? 'diff-viewer__mode-btn--active' : ''}`}
            onClick={() => setViewMode('unified')}
          >
            Unified
          </button>
          <button
            className={`diff-viewer__mode-btn ${viewMode === 'split' ? 'diff-viewer__mode-btn--active' : ''}`}
            onClick={() => setViewMode('split')}
          >
            Split
          </button>
        </div>
      </div>

      <div className="diff-viewer__legend">
        <span className="diff-viewer__legend-item diff-viewer__legend-item--removed">
          <span className="diff-viewer__legend-marker">-</span> Generated (Original)
        </span>
        <span className="diff-viewer__legend-item diff-viewer__legend-item--added">
          <span className="diff-viewer__legend-marker">+</span> Your Changes (Current)
        </span>
      </div>

      {viewMode === 'unified' ? (
        <UnifiedDiffView diffLines={diffLines} />
      ) : (
        <SplitDiffView diffLines={diffLines} />
      )}
    </div>
  );
}

interface UnifiedDiffViewProps {
  diffLines: DiffLine[];
}

function UnifiedDiffView({ diffLines }: UnifiedDiffViewProps) {
  return (
    <div className="diff-viewer__content diff-viewer__content--unified">
      <pre className="diff-viewer__code">
        {diffLines.map((line, index) => (
          <div
            key={index}
            className={`diff-line diff-line--${line.type}`}
          >
            <span className="diff-line__gutter">
              <span className="diff-line__num diff-line__num--original">
                {line.originalLineNum ?? ''}
              </span>
              <span className="diff-line__num diff-line__num--current">
                {line.currentLineNum ?? ''}
              </span>
              <span className="diff-line__marker">
                {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
              </span>
            </span>
            <span className="diff-line__content">{line.content || ' '}</span>
          </div>
        ))}
      </pre>
    </div>
  );
}

interface SplitDiffViewProps {
  diffLines: DiffLine[];
}

function SplitDiffView({ diffLines }: SplitDiffViewProps) {
  // Organize lines for split view
  const { leftLines, rightLines } = useMemo(() => {
    const left: (DiffLine | null)[] = [];
    const right: (DiffLine | null)[] = [];

    for (const line of diffLines) {
      if (line.type === 'unchanged') {
        left.push(line);
        right.push(line);
      } else if (line.type === 'removed') {
        left.push(line);
        right.push(null);
      } else if (line.type === 'added') {
        // Check if previous right was null (we can fill it)
        if (right.length > 0 && right[right.length - 1] === null) {
          right[right.length - 1] = line;
        } else {
          left.push(null);
          right.push(line);
        }
      }
    }

    return { leftLines: left, rightLines: right };
  }, [diffLines]);

  return (
    <div className="diff-viewer__content diff-viewer__content--split">
      <div className="diff-viewer__split-pane diff-viewer__split-pane--left">
        <div className="diff-viewer__pane-header">Generated (Original)</div>
        <pre className="diff-viewer__code">
          {leftLines.map((line, index) => (
            <div
              key={index}
              className={`diff-line ${line ? `diff-line--${line.type}` : 'diff-line--empty'}`}
            >
              <span className="diff-line__gutter">
                <span className="diff-line__num">{line?.originalLineNum ?? ''}</span>
                <span className="diff-line__marker">
                  {line?.type === 'removed' ? '-' : ' '}
                </span>
              </span>
              <span className="diff-line__content">{line?.content || ' '}</span>
            </div>
          ))}
        </pre>
      </div>

      <div className="diff-viewer__split-pane diff-viewer__split-pane--right">
        <div className="diff-viewer__pane-header">Your Changes (Current)</div>
        <pre className="diff-viewer__code">
          {rightLines.map((line, index) => (
            <div
              key={index}
              className={`diff-line ${line ? `diff-line--${line.type}` : 'diff-line--empty'}`}
            >
              <span className="diff-line__gutter">
                <span className="diff-line__num">{line?.currentLineNum ?? ''}</span>
                <span className="diff-line__marker">
                  {line?.type === 'added' ? '+' : ' '}
                </span>
              </span>
              <span className="diff-line__content">{line?.content || ' '}</span>
            </div>
          ))}
        </pre>
      </div>
    </div>
  );
}

export default DiffViewer;
