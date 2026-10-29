import clsx from 'clsx';
import { type ComponentProps, type ReactNode, useId, useState } from 'react';

import { clamp, parseNumber } from './lib/number';

type ButtonProps = ComponentProps<'button'> & { variant?: 'primary' | 'quiet' | 'danger' };

export function Button({ variant = 'quiet', className, type = 'button', ...rest }: ButtonProps) {
  return (
    <button
      type={type}
      className={clsx(
        'inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-[13px] font-medium whitespace-nowrap transition-colors disabled:cursor-not-allowed disabled:opacity-40',
        variant === 'primary' && 'bg-marker text-marker-ink hover:bg-[#ffd04a]',
        variant === 'quiet' && 'border border-line text-ink hover:border-dim hover:bg-sheet',
        variant === 'danger' && 'border border-line text-alert hover:border-alert',
        className,
      )}
      {...rest}
    />
  );
}

const control =
  'w-full rounded-md border border-line bg-rail px-2.5 py-1.5 text-ink placeholder:text-faint focus:border-dim focus:outline-none';

interface FieldProps {
  label: string;
  hint?: string;
  children: (id: string) => ReactNode;
}

/** Label + control + optional hint, wired with a generated id. */
export function Field({ label, hint, children }: FieldProps) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-[13px] text-dim">
        {label}
      </label>
      {children(id)}
      {hint && <p className="text-[12px] text-faint">{hint}</p>}
    </div>
  );
}

export function Input({ className, ...rest }: ComponentProps<'input'>) {
  return <input className={clsx(control, 'h-8', className)} {...rest} />;
}

type NumberInputProps = Omit<ComponentProps<'input'>, 'type' | 'value' | 'onChange' | 'min' | 'max'> & {
  value: number;
  onValueChange: (n: number) => void;
  min?: number;
  max?: number;
};

/**
 * Number field that can be cleared and retyped. The raw text is kept while editing,
 * so an empty field never turns into 0; the value is clamped to [min, max] on blur.
 */
export function NumberInput({ value, onValueChange, min, max, onBlur, ...rest }: NumberInputProps) {
  const [text, setText] = useState<string | null>(null);
  return (
    <Input
      type="number"
      min={min}
      max={max}
      value={text ?? String(value)}
      onChange={(e) => {
        setText(e.target.value);
        const n = parseNumber(e.target.value);
        if (n !== null) onValueChange(n);
      }}
      onBlur={(e) => {
        const n = clamp(value, min, max);
        if (n !== value) onValueChange(n);
        setText(null);
        onBlur?.(e);
      }}
      {...rest}
    />
  );
}

export function TextArea({ className, ...rest }: ComponentProps<'textarea'>) {
  return <textarea className={clsx(control, 'resize-y leading-relaxed', className)} {...rest} />;
}

export function Select({ className, ...rest }: ComponentProps<'select'>) {
  return <select className={clsx(control, 'h-8', className)} {...rest} />;
}

/** Two or three mutually exclusive options shown side by side. */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="inline-flex w-fit rounded-md border border-line p-0.5"
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          onClick={() => {
            onChange(o.value);
          }}
          className={clsx(
            'h-7 rounded px-3 text-[13px]',
            o.value === value ? 'bg-sheet text-ink' : 'text-dim hover:text-ink',
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function ErrorText({ children }: { children: ReactNode }) {
  return (
    <p role="alert" className="text-[13px] whitespace-pre-wrap text-alert">
      {children}
    </p>
  );
}

/** Thin progress line; indeterminate when total is 0. */
export function Progress({ done, total }: { done: number; total: number }) {
  const pct = total > 0 ? Math.min(100, (done / total) * 100) : 0;
  return (
    <div
      className="h-1 overflow-hidden rounded-full bg-line"
      role="progressbar"
      aria-valuenow={Math.round(pct)}
    >
      <div
        className={clsx('h-full bg-marker', total === 0 && 'w-1/3 animate-pulse')}
        style={total > 0 ? { width: `${pct}%` } : undefined}
      />
    </div>
  );
}
