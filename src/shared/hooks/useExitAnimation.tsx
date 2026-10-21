import { useRef, useState } from 'react';

const REDUCED_MOTION = '(prefers-reduced-motion: reduce)';

/**
 * Keeps an element on screen while its exit animation (`animationName`) plays, then runs the callback.
 * Put `leaving` on the element's exit class and `onAnimationEnd` on the element itself.
 * Under reduced motion there is no animation, so the callback runs at once.
 */
export default function useExitAnimation(animationName: string) {
  // ref
  const done = useRef<(() => void) | null>(null);

  // state
  const [leaving, setLeaving] = useState(false);

  // handle
  const exit = (onDone: () => void) => {
    if (window.matchMedia(REDUCED_MOTION).matches) {
      onDone();

      return;
    }
    done.current = onDone;
    setLeaving(true);
  };

  /** Shows the element again, e.g. when the removal it animated failed. */
  const cancel = () => {
    done.current = null;
    setLeaving(false);
  };

  const onAnimationEnd = (e: React.AnimationEvent) => {
    // Child animations bubble up here too.
    if (e.target !== e.currentTarget || e.animationName !== animationName) {
      return;
    }
    const fn = done.current;
    done.current = null;
    fn?.();
  };

  return { leaving, exit, cancel, onAnimationEnd };
}
