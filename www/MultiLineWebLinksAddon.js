/**
 * MultiLineWebLinksAddon - Custom xterm.js addon for handling wrapped URLs
 * Based on @xterm/addon-web-links but detects URLs split across lines
 */

// More permissive URL regex - captures trailing periods/dashes for multiline detection
const URL_REGEX = /(https?):\/\/([^\s"'<>{}]+)/gi;

function handleLink(event, uri) {
  const newWindow = window.open();
  if (newWindow) {
    try {
      newWindow.opener = null;
    } catch {
      // no-op, Electron can throw
    }
    newWindow.location.href = uri;
  } else {
    console.warn('Opening link blocked as opener could not be cleared');
  }
}

function isUrl(urlString) {
  try {
    const url = new URL(urlString);
    const parsedBase = url.password && url.username
      ? `${url.protocol}//${url.username}:${url.password}@${url.host}`
      : url.username
        ? `${url.protocol}//${url.username}@${url.host}`
        : `${url.protocol}//${url.host}`;
    return urlString.toLowerCase().startsWith(parsedBase.toLowerCase());
  } catch (e) {
    return false;
  }
}

class MultiLineWebLinkProvider {
  constructor(terminal, regex, handler, options = {}) {
    this._terminal = terminal;
    this._regex = regex;
    this._handler = handler;
    this._options = options;
  }

  provideLinks(y, callback) {
    const links = this._computeLink(y);
    callback(this._addCallbacks(links));
  }

  _addCallbacks(links) {
    return links.map(link => {
      link.leave = this._options.leave;
      link.hover = (event, uri) => {
        if (this._options.hover) {
          const { range } = link;
          this._options.hover(event, uri, range);
        }
      };
      return link;
    });
  }

  _computeLink(y) {
    const terminal = this._terminal;
    const activate = this._handler;
    const result = [];
    const buf = terminal.buffer.active;

    const currentLine = buf.getLine(y - 1);
    if (!currentLine) {
      return result;
    }

    // Scan backward to find if a URL started on a previous line
    const maxLookBack = 10;
    let scanStartLine = y - 1;
    
    for (let i = 1; i <= maxLookBack; i++) {
      const line = buf.getLine(y - 1 - i);
      if (!line) break;
      const text = line.translateToString(true);
      if (text.trim().length === 0) break; // Stop at empty line
      
      // Check if this line contains http:// or https://
      if (text.match(/(https?):\/\//i)) {
        scanStartLine = y - 1 - i;
        break;
      }
    }

    // Collect lines from URL start forward until we hit empty line
    const lines = [];
    const maxLookAhead = 15;
    
    for (let i = 0; i < maxLookAhead; i++) {
      const line = buf.getLine(scanStartLine + i);
      if (!line) break;
      const text = line.translateToString(true);
      // Stop if we hit an empty or whitespace-only line (unless it's the first line)
      if (i > 0 && text.trim().length === 0) {
        break;
      }
      lines.push({ lineIndex: scanStartLine + i, text });
    }



    // Try to find URLs in all collected lines
    const rex = new RegExp(this._regex.source, this._regex.flags || 'gi');

    for (const lineInfo of lines) {
      const lineText = lineInfo.text;
      const lineIdx = lineInfo.lineIndex;
      let match;
      rex.lastIndex = 0; // Reset regex

      while ((match = rex.exec(lineText)) !== null) {
      const rawMatch = match[0];
      let urlText = rawMatch.replace(/\s+/g, ''); // Remove all whitespace
      let matchLength = rawMatch.length;
      const continuationLines = []; // Track which lines are part of this URL
      
      // Check if this URL is incomplete (ends with - or / or .)
      // Only then try to join with subsequent lines
      if (urlText.endsWith('-') || urlText.endsWith('/') || urlText.endsWith('.')) {
        // Find current line in our collected lines
        const currentIdx = lines.findIndex(l => l.lineIndex === lineIdx);
        if (currentIdx === -1) continue;
        
        // Scan subsequent lines for continuation
        for (let i = 1; i < lines.length - currentIdx; i++) {
          const nextLineInfo = lines[currentIdx + i];
          const nextText = nextLineInfo.text.trim();
          
          if (nextText.length === 0) {
            break;
          }
          
          // Extract URL-like continuation (no protocol)
          const continuationMatch = nextText.match(/^([a-zA-Z0-9\-._~:/?#@!$&'()*+,;=%]+)/);
          if (continuationMatch) {
            const continuation = continuationMatch[1];
            const contLineIdx = lineIdx + i;
            continuationLines.push({ lineIndex: contLineIdx, text: nextLineInfo.text, continuation });
            urlText = urlText + continuation;
            
            // If this continuation doesn't end with - / or ., we're done
            if (!continuation.endsWith('-') && !continuation.endsWith('/') && !continuation.endsWith('.')) {
              break;
            }
          } else {
            break;
          }
        }
      }

      // Validate it's a real URL
      if (!isUrl(urlText)) {
        continue;
      }

      // Create link for the first line (the one with http://)
      const [startY, startX] = this._mapStrIdx(terminal, lineIdx, 0, match.index);
      const [endY, endX] = this._mapStrIdx(terminal, startY, startX, matchLength);

      if (startY === -1 || startX === -1 || endY === -1 || endX === -1) {
        continue;
      }

      // Range expects 1-based, right side including for start, excluding for end
      const range = {
        start: {
          x: startX + 1,
          y: startY + 1
        },
        end: {
          x: endX,
          y: endY + 1
        }
      };

      result.push({ range, text: urlText, activate });

      // Create additional links for continuation lines
      for (const cont of continuationLines) {
        const contLine = buf.getLine(cont.lineIndex);
        if (!contLine) {
          continue;
        }
        
        const contText = cont.text;
        const contStart = contText.indexOf(cont.continuation);
        if (contStart === -1) continue;
        
        const [contStartY, contStartX] = this._mapStrIdx(terminal, cont.lineIndex, 0, contStart);
        const [contEndY, contEndX] = this._mapStrIdx(terminal, contStartY, contStartX, cont.continuation.length);
        
        if (contStartY === -1 || contStartX === -1 || contEndY === -1 || contEndX === -1) {
          continue;
        }
        
        const contRange = {
          start: {
            x: contStartX + 1,
            y: contStartY + 1
          },
          end: {
            x: contEndX,
            y: contEndY + 1
          }
        };
        
        result.push({ range: contRange, text: urlText, activate });
      }
      }
    }

    return result;
  }

  /**
   * Map a string index back to buffer positions.
   * This is the EXACT original logic from WebLinkProvider
   */
  _mapStrIdx(terminal, lineIndex, rowIndex, stringIndex) {
    const buf = terminal.buffer.active;
    const cell = buf.getNullCell();
    let start = rowIndex;
    
    while (stringIndex) {
      const line = buf.getLine(lineIndex);
      if (!line) {
        return [-1, -1];
      }
      
      for (let i = start; i < line.length; ++i) {
        line.getCell(i, cell);
        const chars = cell.getChars();
        const width = cell.getWidth();
        
        if (width) {
          stringIndex -= chars.length || 1;

          // correct stringIndex for early wrapped wide chars:
          // - currently only happens at last cell
          // - cells to the right are reset with chars='' and width=1 in InputHandler.print
          // - follow-up line must be wrapped and contain wide char at first cell
          // --> if all these conditions are met, correct stringIndex by +1
          if (i === line.length - 1 && chars === '') {
            const line = buf.getLine(lineIndex + 1);
            if (line && line.isWrapped) {
              line.getCell(0, cell);
              if (cell.getWidth() === 2) {
                stringIndex += 1;
              }
            }
          }
        }
        
        if (stringIndex < 0) {
          return [lineIndex, i];
        }
      }
      
      lineIndex++;
      start = 0;
    }
    
    return [lineIndex, start];
  }
}

export class MultiLineWebLinksAddon {
  constructor(handler = handleLink, options = {}) {
    this._handler = handler;
    this._options = options;
    this._terminal = undefined;
    this._linkProvider = undefined;
  }

  activate(terminal) {
    this._terminal = terminal;
    const regex = this._options.urlRegex || URL_REGEX;
    this._linkProvider = this._terminal.registerLinkProvider(
      new MultiLineWebLinkProvider(this._terminal, regex, this._handler, this._options)
    );
  }

  dispose() {
    if (this._linkProvider) {
      this._linkProvider.dispose();
    }
  }
}