import poFace from "./assets/po-face.webp";
import poFull from "./assets/po.webp";

export type PoSize = "xs" | "sm" | "md" | "lg" | "xl";

export default function PoAvatar({ size = "md" }: { size?: PoSize }) {
  const faceOnly = size === "xs" || size === "sm";

  return (
    <span className={`po-avatar ${size} ${faceOnly ? "face" : "full"}`} aria-hidden="true">
      <img
        className="po-image"
        src={faceOnly ? poFace : poFull}
        alt=""
        draggable={false}
        decoding="async"
      />
    </span>
  );
}
