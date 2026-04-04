interface FormFieldProps {
  label: string;
  hint?: string;
  children: React.ReactNode;
}

export default function FormField({ label, hint, children }: FormFieldProps) {
  return (
    <div className="form-field">
      <label className="form-label">{label}</label>
      {children}
      {hint && <p className="form-hint">{hint}</p>}
    </div>
  );
}
