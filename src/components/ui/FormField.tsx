interface FormFieldProps {
  label: string;
  hint?: string;
  title?: string;
  children: React.ReactNode;
}

export default function FormField({ label, hint, title, children }: FormFieldProps) {
  return (
    <div className="form-field">
      <label className="form-label" title={title}>
        {label}
        {title && (
          <span style={{
            marginLeft: 4,
            color: "var(--color-text-tertiary)",
            fontSize: 12,
            cursor: "help",
          }}>?</span>
        )}
      </label>
      {children}
      {hint && <p className="form-hint">{hint}</p>}
    </div>
  );
}
