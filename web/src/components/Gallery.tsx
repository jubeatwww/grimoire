import { useEffect, useState } from "react";

interface GalleryProps {
  /// Used to reset the carousel when the displayed item changes.
  itemKey: string;
  cover: string | null;
  samples: string[];
}

export function Gallery({ itemKey, cover, samples }: GalleryProps) {
  const images = (cover ? [cover] : []).concat(samples);
  const [index, setIndex] = useState(0);

  // Reset when item changes — index from a previous item is meaningless.
  useEffect(() => {
    setIndex(0);
  }, [itemKey]);

  // Clamp in case images shrunk (e.g. refresh removed some).
  useEffect(() => {
    if (index >= images.length && images.length > 0) {
      setIndex(images.length - 1);
    }
  }, [images.length, index]);

  // Keyboard ←/→ to cycle. Skip when focus is in a text field.
  useEffect(() => {
    if (images.length <= 1) return;
    const handler = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (
        t &&
        (t.tagName === "INPUT" ||
          t.tagName === "TEXTAREA" ||
          t.tagName === "SELECT" ||
          t.isContentEditable)
      ) {
        return;
      }
      if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        setIndex((i) => (i - 1 + images.length) % images.length);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        setIndex((i) => (i + 1) % images.length);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [images.length]);

  if (images.length === 0) {
    return <div className="gallery gallery-empty">No images yet</div>;
  }

  const current = images[index];
  return (
    <div className="gallery">
      <div className="gallery-main">
        <img src={current} alt="" />
        {images.length > 1 && (
          <>
            <button
              type="button"
              className="gallery-nav gallery-nav-prev"
              onClick={() =>
                setIndex((i) => (i - 1 + images.length) % images.length)
              }
              aria-label="Previous image"
            >
              ‹
            </button>
            <button
              type="button"
              className="gallery-nav gallery-nav-next"
              onClick={() => setIndex((i) => (i + 1) % images.length)}
              aria-label="Next image"
            >
              ›
            </button>
            <span className="gallery-counter">
              {index + 1} / {images.length}
            </span>
          </>
        )}
      </div>
      {images.length > 1 && (
        <div className="gallery-thumbs">
          {images.map((url, i) => (
            <button
              key={`${i}-${url}`}
              type="button"
              className={`gallery-thumb ${i === index ? "active" : ""}`}
              onClick={() => setIndex(i)}
              aria-label={`Image ${i + 1}`}
            >
              <img src={url} alt="" loading="lazy" />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
