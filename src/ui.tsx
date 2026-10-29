import clsx from 'clsx';
import { type ComponentProps, type ReactNode, useId, useState } from 'react';

import { clamp, parseNumber } from './lib/number';

type ButtonProps = ComponentProps<'button'> & { variant?: 'primary' | 'quiet' | 'danger' };

/** Primary is ink on graphite: colour is kept for the user's picks (see app.css), not for buttons. */
export function Button({ variant = 'quiet', className, type = 'button', ...rest }: ButtonProps) {
  return (
    <button
      type={type}
      className={clsx(
        'inline-flex h-8 items-center justify-center gap-1.5 rounded px-3 text-[13px] font-medium whitespace-nowrap transition-colors disabled:cursor-not-allowed disabled:opacity-40',
        variant === 'primary' && 'bg-ink text-rail hover:bg-white',
        variant === 'quiet' && 'border border-edge text-ink hover:bg-sheet',
        variant === 'danger' && 'border border-edge text-alert hover:border-alert',
        className,
      )}
      {...rest}
    />
  );
}

const control =
  'w-full rounded border border-line bg-well px-2.5 py-1.5 text-ink placeholder:text-faint hover:border-edge focus:border-dim focus:outline-none';

/** A screen: one centred frame with a header row (title, optional subtitle, actions) above the body. */
export function Page({
  title,
  subtitle,
  actions,
  children,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="mx-auto flex w-full max-w-[1360px] flex-col gap-7 px-8 pt-7 pb-12">
      <header className="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div className="flex min-w-0 flex-col gap-1">
          <h1 className="truncate text-[21px] leading-tight font-semibold">{title}</h1>
          {subtitle && <p className="text-[13px] text-faint">{subtitle}</p>}
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </header>
      {children}
    </div>
  );
}

/** Row above a list or board: a short status on the left, the actions for it on the right. */
export function Toolbar({ children, actions }: { children?: ReactNode; actions?: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
      <div className="min-w-0 text-[13px] text-dim">{children}</div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}

/** A titled group of fields or facts, with optional actions beside the title. */
export function Panel({
  title,
  hint,
  actions,
  children,
  className,
}: {
  title: string;
  hint?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={clsx('flex flex-col gap-4 rounded-md bg-sheet p-5', className)}>
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-0.5">
          <h2 className="text-[14px] font-semibold">{title}</h2>
          {hint && <p className="text-[12.5px] text-faint">{hint}</p>}
        </div>
        {actions}
      </div>
      {children}
    </section>
  );
}

interface FieldProps {
  label: string;
  hint?: string | undefined;
  children: (id: string) => ReactNode;
}

/** Label + control + optional hint, wired with a generated id. */
export function Field({ label, hint, children }: FieldProps) {
  const id = useId();
  return (
    <div className="flex min-w-0 flex-col gap-1.5">
      <label htmlFor={id} className="text-[12.5px] font-medium text-dim">
        {label}
      </label>
      {children(id)}
      {hint && <p className="text-[12px] leading-snug text-faint">{hint}</p>}
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

/** Checkbox with a label and an optional second line. */
export function Checkbox({
  id,
  checked,
  onChange,
  label,
  hint,
}: {
  id: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  hint?: string;
}) {
  return (
    <label htmlFor={id} className="flex cursor-pointer items-start gap-2.5 text-[13px]">
      <input
        id={id}
        type="checkbox"
        className="mt-0.5 size-4 accent-marker"
        checked={checked}
        onChange={(e) => {
          onChange(e.target.checked);
        }}
      />
      <span className="flex flex-col">
        {label}
        {hint && <span className="text-[12px] text-faint">{hint}</span>}
      </span>
    </label>
  );
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
      className="inline-flex w-fit rounded border border-line bg-well p-0.5"
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
            'h-7 rounded-sm px-3 text-[13px]',
            o.value === value
              ? 'bg-sheet text-ink shadow-[inset_0_-2px_0_var(--color-marker)]'
              : 'text-dim hover:text-ink',
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
        className={clsx('h-full bg-ink', total === 0 && 'w-1/3 animate-pulse')}
        style={total > 0 ? { width: `${String(pct)}%` } : undefined}
      />
    </div>
  );
}
