import { useEffect, useRef, useState } from "react";
import { UiIcon } from "./ui-icon";
import { invoke } from "@tauri-apps/api/core";

export function ProgramIcon({ source, id, name }: { source: string; id: string; name: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [icon, setIcon] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setIcon(null);
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        void invoke<string | null>("program_icon", { source, id })
          .then((value) => {
            if (alive) setIcon(value);
          })
          .catch(() => {});
      },
      { rootMargin: "120px" },
    );
    if (ref.current) observer.observe(ref.current);
    return () => {
      alive = false;
      observer.disconnect();
    };
  }, [source, id]);
  return (
    <span
      ref={ref}
      className={`program-icon ${icon ? "has-native-icon" : "fallback-icon"}`}
      title={name}
      aria-hidden="true"
    >
      {icon ? (
        <img
          src={icon}
          alt=""
          onError={() => {
            setIcon(null);
          }}
        />
      ) : (
        <UiIcon name={source === "store" ? "apps" : "package"} />
      )}
    </span>
  );
}
