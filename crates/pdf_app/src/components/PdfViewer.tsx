import { useEffect, useState, useRef, useCallback, memo } from "react";

interface SvgDocument {
  pages: string[];
  width_pt: number;
  height_pt: number;
}

interface PdfViewerProps {
  svgDoc: SvgDocument | null;
  onSave: () => void;
}

const EDGE_PADDING = 24; // Grace padding when panning at edges
const PAGE_CHANGE_THRESHOLD = 20; // Accumulated scroll needed to change pages (lowered for easier scrolling)
const PAGE_CHANGE_COOLDOWN = 150; // Milliseconds between page changes
const ZOOM_ANIMATION_DURATION = 200; // Duration for animated zoom transitions (ms)

// Memoized SVG renderer to prevent re-renders during pan/zoom
const SvgRenderer = memo(function SvgRenderer({ svg }: { svg: string }) {
  return <div dangerouslySetInnerHTML={{ __html: svg }} />;
});

export function PdfViewer({ svgDoc, onSave }: PdfViewerProps) {
  const [currentPage, setCurrentPage] = useState(1);
  const [scale, setScale] = useState(1);
  const [translate, setTranslate] = useState({ x: 0, y: 0 });
  const [isFitMode, setIsFitMode] = useState(true); // Start in fit mode
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const lastPointer = useRef({ x: 0, y: 0 });
  const currentPageRef = useRef(currentPage);
  const pageChangeAccumulator = useRef(0); // Accumulated scroll past edge
  const lastPageChangeTime = useRef(0); // For cooldown between page changes
  const zoomAnimationRef = useRef<number | null>(null); // For animated zoom

  // Refs to hold current values for event handlers (avoids stale closures)
  const scaleRef = useRef(scale);
  const translateRef = useRef(translate);
  const svgDocRef = useRef(svgDoc);
  const isFitModeRef = useRef(isFitMode);

  // Keep refs in sync with state
  useEffect(() => {
    scaleRef.current = scale;
  }, [scale]);

  useEffect(() => {
    translateRef.current = translate;
  }, [translate]);

  useEffect(() => {
    svgDocRef.current = svgDoc;
  }, [svgDoc]);

  useEffect(() => {
    currentPageRef.current = currentPage;
  }, [currentPage]);

  useEffect(() => {
    isFitModeRef.current = isFitMode;
  }, [isFitMode]);

  const numPages = svgDoc?.pages.length ?? 0;

  // Get viewport dimensions
  const getViewport = useCallback(() => {
    if (!containerRef.current) return { width: 0, height: 0 };
    return {
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
    };
  }, []);

  // Check if document overflows viewport in a given direction
  const getOverflowState = useCallback(
    (currentScale: number, currentTranslate: { x: number; y: number }) => {
      const viewport = getViewport();
      const doc = svgDocRef.current;
      if (!doc)
        return {
          canPanUp: false,
          canPanDown: false,
          canPanLeft: false,
          canPanRight: false,
        };

      const docWidth = doc.width_pt * currentScale;
      const docHeight = doc.height_pt * currentScale;

      const overflowsHorizontally = docWidth > viewport.width;
      const overflowsVertically = docHeight > viewport.height;

      // Calculate if we're at the edges
      const atTop = currentTranslate.y >= EDGE_PADDING;
      const atBottom =
        currentTranslate.y <= viewport.height - docHeight - EDGE_PADDING;
      const atLeft = currentTranslate.x >= EDGE_PADDING;
      const atRight =
        currentTranslate.x <= viewport.width - docWidth - EDGE_PADDING;

      return {
        canPanUp: overflowsVertically && !atTop,
        canPanDown: overflowsVertically && !atBottom,
        canPanLeft: overflowsHorizontally && !atLeft,
        canPanRight: overflowsHorizontally && !atRight,
      };
    },
    [getViewport],
  );

  // Clamp translation to keep document centered when small, or within bounds when large
  const clampTranslate = useCallback(
    (
      x: number,
      y: number,
      currentScale: number,
      doc: SvgDocument | null = svgDocRef.current,
    ) => {
      const viewport = getViewport();
      if (!doc) return { x: 0, y: 0 };

      const docWidth = doc.width_pt * currentScale;
      const docHeight = doc.height_pt * currentScale;

      let clampedX = x;
      let clampedY = y;

      // Horizontal: center if doc fits, otherwise clamp with padding
      if (docWidth <= viewport.width) {
        clampedX = (viewport.width - docWidth) / 2;
      } else {
        const minX = viewport.width - docWidth - EDGE_PADDING;
        const maxX = EDGE_PADDING;
        clampedX = Math.min(maxX, Math.max(minX, x));
      }

      // Vertical: center if doc fits, otherwise clamp with padding
      if (docHeight <= viewport.height) {
        clampedY = (viewport.height - docHeight) / 2;
      } else {
        const minY = viewport.height - docHeight - EDGE_PADDING;
        const maxY = EDGE_PADDING;
        clampedY = Math.min(maxY, Math.max(minY, y));
      }

      return { x: clampedX, y: clampedY };
    },
    [getViewport],
  );

  // Fit document to viewport and center it (returns target values for animation)
  const getFitValues = useCallback(() => {
    const viewport = getViewport();
    const doc = svgDocRef.current;
    if (!doc || viewport.width === 0 || viewport.height === 0) return null;

    // Calculate scale to fit with padding
    const padding = EDGE_PADDING * 2;
    const scaleX = (viewport.width - padding) / doc.width_pt;
    const scaleY = (viewport.height - padding) / doc.height_pt;
    const newScale = Math.min(scaleX, scaleY); // Fit to viewport (allow upscaling)

    const docWidth = doc.width_pt * newScale;
    const docHeight = doc.height_pt * newScale;

    const x = (viewport.width - docWidth) / 2;
    const y = (viewport.height - docHeight) / 2;

    return { scale: newScale, x, y };
  }, [getViewport]);

  // Fit without animation (for initial load, resize, etc.)
  const fitToViewport = useCallback(() => {
    const fit = getFitValues();
    if (fit) {
      setScale(fit.scale);
      setTranslate({ x: fit.x, y: fit.y });
    }
  }, [getFitValues]);

  // Animated zoom to target scale and position
  const animateZoom = useCallback(
    (
      targetScale: number,
      targetX: number,
      targetY: number,
      duration: number = ZOOM_ANIMATION_DURATION,
    ) => {
      // Cancel any existing animation
      if (zoomAnimationRef.current) {
        cancelAnimationFrame(zoomAnimationRef.current);
      }

      const startScale = scaleRef.current;
      const startX = translateRef.current.x;
      const startY = translateRef.current.y;
      const startTime = performance.now();

      const animate = (currentTime: number) => {
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / duration, 1);

        // Ease out cubic for smooth deceleration
        const eased = 1 - Math.pow(1 - progress, 3);

        const currentScale = startScale + (targetScale - startScale) * eased;
        const currentX = startX + (targetX - startX) * eased;
        const currentY = startY + (targetY - startY) * eased;

        setScale(currentScale);
        setTranslate({ x: currentX, y: currentY });

        if (progress < 1) {
          zoomAnimationRef.current = requestAnimationFrame(animate);
        } else {
          zoomAnimationRef.current = null;
        }
      };

      zoomAnimationRef.current = requestAnimationFrame(animate);
    },
    [],
  );

  // Reset to page 1 if current page exceeds new page count
  useEffect(() => {
    if (currentPage > numPages && numPages > 0) {
      setCurrentPage(1);
    }
  }, [numPages, currentPage]);

  // Fit on initial load and when document changes
  useEffect(() => {
    if (svgDoc) {
      requestAnimationFrame(() => fitToViewport());
    }
  }, [svgDoc, fitToViewport]);

  // Handle window resize - re-fit if in fit mode, otherwise re-clamp
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver(() => {
      if (isFitMode) {
        fitToViewport();
      } else {
        // Re-clamp translate to keep document properly positioned
        const currentScale = scaleRef.current;
        const currentTranslate = translateRef.current;
        const clamped = clampTranslate(
          currentTranslate.x,
          currentTranslate.y,
          currentScale,
        );
        setTranslate(clamped);
      }
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, [clampTranslate, isFitMode, fitToViewport]);

  // Handle pinch zoom and scroll/pan
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();

      if (e.ctrlKey) {
        // Pinch to zoom - disable fit mode and cancel any animation
        setIsFitMode(false);
        if (zoomAnimationRef.current) {
          cancelAnimationFrame(zoomAnimationRef.current);
          zoomAnimationRef.current = null;
        }

        const rect = container.getBoundingClientRect();
        const pointerX = e.clientX - rect.left;
        const pointerY = e.clientY - rect.top;

        const currentScale = scaleRef.current;
        const currentTranslate = translateRef.current;

        // Use exponential scaling for smoother feel
        // deltaY is typically around 1-10 for trackpad pinch
        const zoomFactor = Math.pow(0.995, e.deltaY);
        const newScale = Math.min(4, Math.max(0.1, currentScale * zoomFactor));

        // Zoom toward pointer position
        const scaleRatio = newScale / currentScale;
        const newX = pointerX - (pointerX - currentTranslate.x) * scaleRatio;
        const newY = pointerY - (pointerY - currentTranslate.y) * scaleRatio;

        const clamped = clampTranslate(newX, newY, newScale);
        setScale(newScale);
        setTranslate(clamped);
      } else {
        // Two-finger scroll: pan if possible, otherwise navigate pages
        const currentScale = scaleRef.current;
        const currentTranslate = translateRef.current;
        const overflow = getOverflowState(currentScale, currentTranslate);

        // Check if this is a discrete scroll (mouse wheel with clicks)
        // deltaMode 1 = DOM_DELTA_LINE (discrete), deltaMode 0 = DOM_DELTA_PIXEL (continuous/trackpad)
        const isDiscreteScroll = e.deltaMode === 1;

        // Try to pan first
        let didPan = false;
        let newX = currentTranslate.x;
        let newY = currentTranslate.y;

        // Horizontal panning
        if (e.deltaX !== 0) {
          if (
            (e.deltaX > 0 && overflow.canPanRight) ||
            (e.deltaX < 0 && overflow.canPanLeft)
          ) {
            newX = currentTranslate.x - e.deltaX * (isDiscreteScroll ? 20 : 1);
            didPan = true;
          }
        }

        // Vertical panning
        if (e.deltaY !== 0) {
          if (
            (e.deltaY > 0 && overflow.canPanDown) ||
            (e.deltaY < 0 && overflow.canPanUp)
          ) {
            newY = currentTranslate.y - e.deltaY * (isDiscreteScroll ? 20 : 1);
            didPan = true;
            // Reset accumulator when panning
            pageChangeAccumulator.current = 0;
          }
        }

        if (didPan) {
          setTranslate(clampTranslate(newX, newY, currentScale));
        } else if (e.deltaY !== 0) {
          // Can't pan vertically, navigate pages
          const numPages = svgDocRef.current?.pages.length ?? 1;
          const current = currentPageRef.current;
          const now = Date.now();

          // For discrete scroll (mouse wheel), change page immediately on each click
          if (isDiscreteScroll) {
            const cooldownPassed =
              now - lastPageChangeTime.current > PAGE_CHANGE_COOLDOWN;
            if (cooldownPassed) {
              const direction = e.deltaY > 0 ? 1 : -1;
              const newPage = current + direction;
              if (newPage >= 1 && newPage <= numPages) {
                lastPageChangeTime.current = now;
                setCurrentPage(newPage);
              }
            }
          } else {
            // For continuous scroll (trackpad), accumulate and use threshold
            pageChangeAccumulator.current += e.deltaY;

            const cooldownPassed =
              now - lastPageChangeTime.current > PAGE_CHANGE_COOLDOWN;

            if (
              Math.abs(pageChangeAccumulator.current) >=
                PAGE_CHANGE_THRESHOLD &&
              cooldownPassed
            ) {
              const direction = pageChangeAccumulator.current > 0 ? 1 : -1;
              const newPage = current + direction;

              if (newPage >= 1 && newPage <= numPages) {
                lastPageChangeTime.current = now;
                pageChangeAccumulator.current = 0;
                setCurrentPage(newPage);

                // Reset position on new page
                const viewport = getViewport();
                const doc = svgDocRef.current;
                if (doc) {
                  const docHeight = doc.height_pt * currentScale;
                  if (docHeight > viewport.height) {
                    if (direction > 0) {
                      // Going forward: start at top
                      setTranslate((t) =>
                        clampTranslate(t.x, EDGE_PADDING, currentScale),
                      );
                    } else {
                      // Going backward: start at bottom
                      const bottomY =
                        viewport.height - docHeight - EDGE_PADDING;
                      setTranslate((t) =>
                        clampTranslate(t.x, bottomY, currentScale),
                      );
                    }
                  }
                }
              } else {
                // At boundary, reset accumulator
                pageChangeAccumulator.current = 0;
              }
            }
          }
        }
      }
    };

    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, [clampTranslate, getOverflowState, getViewport]);

  // Handle panning with pointer/mouse drag
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handlePointerDown = (e: PointerEvent) => {
      if (e.button !== 0) return;
      // Only start dragging if clicking directly on the container or the document
      const target = e.target as HTMLElement;
      if (target.closest("button")) return;

      isDragging.current = true;
      lastPointer.current = { x: e.clientX, y: e.clientY };
      container.setPointerCapture(e.pointerId);
    };

    const handlePointerMove = (e: PointerEvent) => {
      if (!isDragging.current) return;

      const dx = e.clientX - lastPointer.current.x;
      const dy = e.clientY - lastPointer.current.y;
      lastPointer.current = { x: e.clientX, y: e.clientY };

      const currentTranslate = translateRef.current;
      const currentScale = scaleRef.current;

      const newX = currentTranslate.x + dx;
      const newY = currentTranslate.y + dy;
      setTranslate(clampTranslate(newX, newY, currentScale));
    };

    const handlePointerUp = (e: PointerEvent) => {
      isDragging.current = false;
      container.releasePointerCapture(e.pointerId);
    };

    container.addEventListener("pointerdown", handlePointerDown);
    container.addEventListener("pointermove", handlePointerMove);
    container.addEventListener("pointerup", handlePointerUp);
    container.addEventListener("pointercancel", handlePointerUp);

    return () => {
      container.removeEventListener("pointerdown", handlePointerDown);
      container.removeEventListener("pointermove", handlePointerMove);
      container.removeEventListener("pointerup", handlePointerUp);
      container.removeEventListener("pointercancel", handlePointerUp);
    };
  }, [clampTranslate]);

  const goToPrevPage = useCallback(() => {
    setCurrentPage((p) => Math.max(p - 1, 1));
  }, []);

  const goToNextPage = useCallback(() => {
    setCurrentPage((p) =>
      Math.min(p + 1, svgDocRef.current?.pages.length ?? 1),
    );
  }, []);

  const handleZoomIn = useCallback(() => {
    setIsFitMode(false);
    const currentScale = scaleRef.current;
    const currentTranslate = translateRef.current;
    const newScale = Math.min(4, currentScale * 1.25);
    const viewport = getViewport();

    const centerX = viewport.width / 2;
    const centerY = viewport.height / 2;
    const scaleRatio = newScale / currentScale;
    const newX = centerX - (centerX - currentTranslate.x) * scaleRatio;
    const newY = centerY - (centerY - currentTranslate.y) * scaleRatio;

    const clamped = clampTranslate(newX, newY, newScale);
    animateZoom(newScale, clamped.x, clamped.y);
  }, [getViewport, clampTranslate, animateZoom]);

  const handleZoomOut = useCallback(() => {
    setIsFitMode(false);
    const currentScale = scaleRef.current;
    const currentTranslate = translateRef.current;
    const newScale = Math.max(0.1, currentScale / 1.25);
    const viewport = getViewport();

    const centerX = viewport.width / 2;
    const centerY = viewport.height / 2;
    const scaleRatio = newScale / currentScale;
    const newX = centerX - (centerX - currentTranslate.x) * scaleRatio;
    const newY = centerY - (centerY - currentTranslate.y) * scaleRatio;

    const clamped = clampTranslate(newX, newY, newScale);
    animateZoom(newScale, clamped.x, clamped.y);
  }, [getViewport, clampTranslate, animateZoom]);

  const handleFit = useCallback(() => {
    setIsFitMode(true);
    const fit = getFitValues();
    if (fit) {
      animateZoom(fit.scale, fit.x, fit.y);
    }
  }, [getFitValues, animateZoom]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle if viewer is focused or no input is focused
      const activeElement = document.activeElement;
      const isInputFocused =
        activeElement instanceof HTMLTextAreaElement ||
        activeElement instanceof HTMLInputElement;
      if (isInputFocused) return;

      const isMeta = e.metaKey || e.ctrlKey;

      // Arrow keys: page navigation
      if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        setCurrentPage((p) => Math.max(p - 1, 1));
        return;
      }
      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        setCurrentPage((p) =>
          Math.min(p + 1, svgDocRef.current?.pages.length ?? 1),
        );
        return;
      }

      // Home/End: first/last page
      if (e.key === "Home") {
        e.preventDefault();
        setCurrentPage(1);
        return;
      }
      if (e.key === "End") {
        e.preventDefault();
        setCurrentPage(svgDocRef.current?.pages.length ?? 1);
        return;
      }

      // Cmd+Plus: zoom in
      if (isMeta && (e.key === "=" || e.key === "+")) {
        e.preventDefault();
        handleZoomIn();
        return;
      }

      // Cmd+Minus: zoom out
      if (isMeta && e.key === "-") {
        e.preventDefault();
        handleZoomOut();
        return;
      }

      // Cmd+0: fit to viewport
      if (isMeta && e.key === "0") {
        e.preventDefault();
        handleFit();
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleZoomIn, handleZoomOut, handleFit]);

  const currentSvg = svgDoc?.pages[currentPage - 1];

  // Calculate scaled dimensions for native SVG rendering (no CSS transform scaling)
  const scaledWidth = svgDoc ? svgDoc.width_pt * scale : 0;
  const scaledHeight = svgDoc ? svgDoc.height_pt * scale : 0;

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full overflow-hidden select-none"
      style={{ backgroundColor: "#1d2021", touchAction: "none" }}
    >
      {/* Page Navigation Bubble */}
      <div
        className="absolute bottom-4 left-4 z-10 flex items-center gap-1 px-2 py-1.5 backdrop-blur-md rounded-full shadow-lg"
        style={{ backgroundColor: "rgba(40, 40, 40, 0.9)", color: "#ebdbb2" }}
      >
        <button
          onClick={goToPrevPage}
          disabled={currentPage <= 1}
          className="w-7 h-7 flex items-center justify-center rounded-full hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="15 18 9 12 15 6" />
          </svg>
        </button>
        <span
          className="text-xs min-w-12 text-center tabular-nums"
          style={{ color: "#a89984" }}
        >
          {numPages > 0 ? currentPage : 0} / {numPages}
        </span>
        <button
          onClick={goToNextPage}
          disabled={currentPage >= numPages}
          className="w-7 h-7 flex items-center justify-center rounded-full hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </button>
      </div>

      {/* Zoom & Save Bubble */}
      <div
        className="absolute bottom-4 right-4 z-10 flex items-center gap-1 px-2 py-1.5 backdrop-blur-md rounded-full shadow-lg"
        style={{ backgroundColor: "rgba(40, 40, 40, 0.9)", color: "#ebdbb2" }}
      >
        <button
          onClick={handleZoomOut}
          className="w-7 h-7 flex items-center justify-center rounded-full hover:bg-white/10 transition-colors"
          title="Zoom out"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
            <line x1="8" y1="11" x2="14" y2="11" />
          </svg>
        </button>
        <button
          onClick={handleFit}
          disabled={isFitMode}
          className={`w-7 h-7 flex items-center justify-center rounded-full transition-colors ${
            isFitMode ? "opacity-30 cursor-default" : "hover:bg-white/10"
          }`}
          title="Fit to window"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
            <text
              x="11"
              y="14"
              textAnchor="middle"
              fontSize="8"
              fill="currentColor"
              stroke="none"
            >
              1
            </text>
          </svg>
        </button>
        <button
          onClick={handleZoomIn}
          className="w-7 h-7 flex items-center justify-center rounded-full hover:bg-white/10 transition-colors"
          title="Zoom in"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
            <line x1="11" y1="8" x2="11" y2="14" />
            <line x1="8" y1="11" x2="14" y2="11" />
          </svg>
        </button>
        <div className="w-px h-4 mx-1" style={{ backgroundColor: "#504945" }} />
        <button
          onClick={onSave}
          disabled={!svgDoc}
          className="w-7 h-7 flex items-center justify-center rounded-full hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
          title="Save PDF"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
        </button>
      </div>

      {/* Document */}
      {currentSvg ? (
        <div
          className="bg-white absolute [&>div>svg]:w-full [&>div>svg]:h-full [&>div>svg]:block rounded-sm"
          style={{
            width: scaledWidth,
            height: scaledHeight,
            transform: `translate(${translate.x}px, ${translate.y}px)`,
            boxShadow:
              "0 4px 24px rgba(0, 0, 0, 0.3), 0 1px 4px rgba(0, 0, 0, 0.2)",
          }}
        >
          <SvgRenderer svg={currentSvg} />
        </div>
      ) : (
        <div
          className="absolute inset-0 flex items-center justify-center"
          style={{ color: "#665c54" }}
        >
          Start typing to see the preview
        </div>
      )}
    </div>
  );
}
