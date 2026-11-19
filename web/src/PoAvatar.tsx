import { useId } from "react";

export type PoSize = "xs" | "sm" | "md" | "lg" | "xl";

export default function PoAvatar({ size = "md" }: { size?: PoSize }) {
  const uid = useId().replace(/:/g, "");
  const bodyGradient = `po-body-${uid}`;
  const bellyGradient = `po-belly-${uid}`;
  const wingGradient = `po-wing-${uid}`;

  return (
    <span className={`po-avatar ${size}`} aria-hidden="true">
      <svg className="po-svg" viewBox="0 0 180 190" focusable="false">
        <defs>
          <linearGradient id={bodyGradient} x1="42" y1="30" x2="135" y2="170" gradientUnits="userSpaceOnUse">
            <stop offset="0" stopColor="var(--po-body-light)" />
            <stop offset="0.55" stopColor="var(--po-body)" />
            <stop offset="1" stopColor="var(--po-body-deep)" />
          </linearGradient>
          <radialGradient id={bellyGradient} cx="0" cy="0" r="1" gradientTransform="translate(90 124) rotate(90) scale(39 35)" gradientUnits="userSpaceOnUse">
            <stop offset="0" stopColor="var(--po-cream)" />
            <stop offset="0.72" stopColor="var(--po-cream)" />
            <stop offset="1" stopColor="var(--po-cream-deep)" />
          </radialGradient>
          <linearGradient id={wingGradient} x1="24" y1="94" x2="54" y2="141" gradientUnits="userSpaceOnUse">
            <stop offset="0" stopColor="var(--po-coral)" />
            <stop offset="1" stopColor="var(--po-coral-deep)" />
          </linearGradient>
        </defs>

        <ellipse className="po-ground" cx="90" cy="180" rx="43" ry="7" fill="var(--po-shadow)" />

        <g className="po-tail">
          <path
            d="M125 138c27-5 39 6 38 18 0 12-12 18-19 11-6-6 0-14 8-12"
            fill="none"
            stroke="var(--po-body-deep)"
            strokeWidth="15"
            strokeLinecap="round"
          />
          <path
            d="M125 138c27-5 39 6 38 18 0 12-12 18-19 11-6-6 0-14 8-12"
            fill="none"
            stroke="var(--po-body-light)"
            strokeWidth="7"
            strokeLinecap="round"
            opacity="0.45"
          />
          <path
            className="po-tail-heart"
            d="M160 147c-5-5-12 0-8 6 2 4 8 8 8 8s6-4 8-8c4-6-3-11-8-6Z"
            fill="var(--po-coral)"
          />
        </g>

        <g className="po-antenna">
          <path
            d="M90 34c2-11-4-15 1-23"
            fill="none"
            stroke="var(--po-antenna-stem)"
            strokeWidth="4.5"
            strokeLinecap="round"
          />
          <path
            d="M91 9c-4-4-11-1-9 4 1 4 9 9 9 9s8-5 9-9c2-5-5-8-9-4Z"
            fill="var(--po-coral)"
          />
          <circle cx="91" cy="14" r="1.7" fill="var(--po-cream)" opacity="0.72" />
        </g>

        <g className="po-wing po-wing-left">
          <path
            d="M48 104C32 89 16 98 19 118c2 15 18 24 34 14Z"
            fill={`url(#${wingGradient})`}
            stroke="var(--po-coral-deep)"
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
          <path d="M31 116c8-4 14-2 19 6" fill="none" stroke="var(--po-cream)" strokeWidth="2" strokeLinecap="round" opacity="0.55" />
        </g>
        <g className="po-wing po-wing-right">
          <path
            d="M132 104c16-15 32-6 29 14-2 15-18 24-34 14Z"
            fill={`url(#${wingGradient})`}
            stroke="var(--po-coral-deep)"
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
          <path d="M149 116c-8-4-14-2-19 6" fill="none" stroke="var(--po-cream)" strokeWidth="2" strokeLinecap="round" opacity="0.55" />
        </g>

        <g className="po-ear po-ear-left">
          <ellipse cx="44" cy="80" rx="13" ry="19" transform="rotate(-25 44 80)" fill="var(--po-body)" stroke="var(--po-body-deep)" strokeWidth="1.4" />
          <ellipse cx="45" cy="81" rx="6.5" ry="11" transform="rotate(-25 45 81)" fill="var(--po-cream)" opacity="0.85" />
        </g>
        <g className="po-ear po-ear-right">
          <ellipse cx="136" cy="80" rx="13" ry="19" transform="rotate(25 136 80)" fill="var(--po-body)" stroke="var(--po-body-deep)" strokeWidth="1.4" />
          <ellipse cx="135" cy="81" rx="6.5" ry="11" transform="rotate(25 135 81)" fill="var(--po-cream)" opacity="0.85" />
        </g>

        <g className="po-feet">
          <ellipse cx="66" cy="172" rx="17" ry="8" fill="var(--po-body-deep)" />
          <ellipse cx="114" cy="172" rx="17" ry="8" fill="var(--po-body-deep)" />
          <path d="M60 171h5M68 171h5M110 171h5M118 171h5" stroke="var(--po-cream)" strokeWidth="1.4" strokeLinecap="round" opacity="0.7" />
        </g>

        <path
          className="po-body-art"
          d="M90 26c29-2 53 18 56 51 2 19-4 35-4 52-2 24-22 41-52 41s-50-17-52-41c0-17-6-33-4-52 3-33 27-53 56-51Z"
          fill={`url(#${bodyGradient})`}
          stroke="var(--po-body-deep)"
          strokeWidth="1.5"
        />
        <ellipse cx="66" cy="53" rx="23" ry="13" transform="rotate(-18 66 53)" fill="var(--po-cream)" opacity="0.2" />
        <ellipse cx="90" cy="124" rx="37" ry="34" fill={`url(#${bellyGradient})`} opacity="0.97" />
        <path d="M78 138c7 3 17 3 24 0" fill="none" stroke="var(--po-cream-deep)" strokeWidth="1.4" strokeLinecap="round" opacity="0.8" />

        <g className="po-arms">
          <path
            d="M42 119c-14-2-23 10-16 22 5 9 16 6 21-4Z"
            fill="var(--po-body)"
            stroke="var(--po-body-deep)"
            strokeWidth="1.4"
          />
          <path
            d="M138 119c14-2 23 10 16 22-5 9-16 6-21-4Z"
            fill="var(--po-body)"
            stroke="var(--po-body-deep)"
            strokeWidth="1.4"
          />
          <ellipse cx="31" cy="139" rx="5.4" ry="3.6" fill="var(--po-cream)" opacity="0.8" />
          <ellipse cx="149" cy="139" rx="5.4" ry="3.6" fill="var(--po-cream)" opacity="0.8" />
        </g>

        <g className="po-face">
          <ellipse cx="90" cy="109" rx="27" ry="18" fill="var(--po-cream)" opacity="0.94" />

          <path d="M63 78c5-4 11-4 15-1" fill="none" stroke="var(--po-ink)" strokeWidth="2" strokeLinecap="round" opacity="0.45" />
          <path d="M102 77c5-3 11-2 15 2" fill="none" stroke="var(--po-ink)" strokeWidth="2" strokeLinecap="round" opacity="0.45" />

          <g className="po-eyes">
            <ellipse className="po-eye po-eye-left" cx="72" cy="87" rx="7.5" ry="9.5" fill="var(--po-ink)" />
            <ellipse className="po-eye po-eye-right" cx="108" cy="87" rx="7.5" ry="9.5" fill="var(--po-ink)" />
            <circle cx="74.5" cy="84" r="2.5" fill="var(--po-cream)" />
            <circle cx="110.5" cy="84" r="2.5" fill="var(--po-cream)" />
            <circle cx="70" cy="90" r="1.2" fill="var(--po-cream)" opacity="0.6" />
            <circle cx="106" cy="90" r="1.2" fill="var(--po-cream)" opacity="0.6" />
          </g>

          <circle className="po-cheek" cx="52" cy="105" r="6.5" fill="var(--po-coral)" opacity="0.2" />
          <circle className="po-cheek" cx="128" cy="105" r="6.5" fill="var(--po-coral)" opacity="0.2" />

          <path
            d="M86 102c2-2 6-2 8 0 1 2-1 4-4 4s-5-2-4-4Z"
            fill="var(--po-coral-deep)"
          />
          <path d="M82 109c4 6 12 7 16 0" fill="none" stroke="var(--po-ink)" strokeWidth="2.2" strokeLinecap="round" />
          <ellipse cx="90" cy="115" rx="4.4" ry="2" fill="var(--po-tongue)" opacity="0.9" />
        </g>

        <path d="M69 64c5-4 11-4 15-1" fill="none" stroke="var(--po-cream)" strokeWidth="1.7" strokeLinecap="round" opacity="0.45" />
      </svg>
    </span>
  );
}
