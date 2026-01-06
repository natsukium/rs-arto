/**
 * Context menu handler for markdown content viewer.
 * Detects the type of element that was right-clicked and reports to Rust.
 */

export type ContentContextType =
  | { type: "general" }
  | { type: "link"; href: string }
  | { type: "image"; src: string; alt: string | null }
  | { type: "code_block"; content: string; language: string | null }
  | { type: "mermaid"; source: string };

export interface ContextMenuData {
  context: ContentContextType;
  x: number;
  y: number;
  has_selection: boolean;
  selected_text: string;
}

/**
 * Detect the context of a right-click by walking up the DOM tree
 */
function detectContext(target: HTMLElement): ContentContextType {
  let current: HTMLElement | null = target;

  while (current && !current.classList.contains("markdown-body")) {
    // Check for mermaid diagram
    if (current.classList.contains("preprocessed-mermaid")) {
      const sourceEl = current.querySelector<HTMLPreElement>(".mermaid-source");
      const source = sourceEl?.textContent || "";
      return { type: "mermaid", source };
    }

    // Check for code block (pre > code)
    if (current.tagName === "PRE" && current.querySelector("code")) {
      const codeEl = current.querySelector("code");
      const content = codeEl?.textContent || "";
      const language = extractLanguage(codeEl);
      return { type: "code_block", content, language };
    }

    // Check for inline code that's part of a code block
    if (current.tagName === "CODE" && current.parentElement?.tagName === "PRE") {
      const content = current.textContent || "";
      const language = extractLanguage(current);
      return { type: "code_block", content, language };
    }

    // Check for image
    if (current.tagName === "IMG") {
      const img = current as HTMLImageElement;
      return {
        type: "image",
        src: img.src,
        alt: img.alt || null,
      };
    }

    // Check for link (but not markdown-link which is handled differently)
    if (current.tagName === "A" && !current.classList.contains("markdown-link")) {
      const anchor = current as HTMLAnchorElement;
      return { type: "link", href: anchor.href };
    }

    // Check for markdown-link (internal links converted by Rust)
    if (current.classList.contains("markdown-link")) {
      const href = current.getAttribute("data-path") || "";
      return { type: "link", href };
    }

    current = current.parentElement;
  }

  return { type: "general" };
}

/**
 * Extract language from code element's class
 */
function extractLanguage(codeEl: HTMLElement | null): string | null {
  if (!codeEl) return null;

  // Look for language-* class
  for (const cls of codeEl.classList) {
    if (cls.startsWith("language-")) {
      return cls.replace("language-", "");
    }
  }

  return null;
}

// Saved selection range for restoration after menu closes
let savedRange: Range | null = null;

/**
 * Get the current text selection and save the range for later restoration
 */
function getTextSelection(): { hasSelection: boolean; selectedText: string } {
  const selection = window.getSelection();
  const selectedText = selection?.toString() ?? "";

  // Save the range for restoration
  if (selection && selection.rangeCount > 0) {
    savedRange = selection.getRangeAt(0).cloneRange();
  } else {
    savedRange = null;
  }

  return {
    hasSelection: selectedText.length > 0,
    selectedText,
  };
}

/**
 * Restore the previously saved selection
 */
export function restoreSelection(): void {
  if (savedRange) {
    const selection = window.getSelection();
    if (selection) {
      selection.removeAllRanges();
      selection.addRange(savedRange);
    }
  }
}

/**
 * Setup context menu event listener on the markdown viewer
 */
export function setup(sendToRust: (data: ContextMenuData) => void): void {
  // Find the markdown body element
  const handler = (event: MouseEvent) => {
    const target = event.target as HTMLElement;

    // Only handle right-clicks within markdown-body
    const markdownBody = target.closest(".markdown-body");
    if (!markdownBody) return;

    // Prevent default browser context menu
    event.preventDefault();

    // Detect context and send to Rust
    const context = detectContext(target);
    const { hasSelection, selectedText } = getTextSelection();
    const data: ContextMenuData = {
      context,
      x: event.clientX,
      y: event.clientY,
      has_selection: hasSelection,
      selected_text: selectedText,
    };

    sendToRust(data);
  };

  // Use capture phase to intercept before other handlers
  document.addEventListener("contextmenu", handler, true);
}
