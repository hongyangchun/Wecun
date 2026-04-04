interface StepIndicatorProps {
  steps: string[];
  currentStep: number;
}

export default function StepIndicator({ steps, currentStep }: StepIndicatorProps) {
  return (
    <div className="stepper">
      {steps.map((label, i) => (
        <div key={label} className="stepper-item">
          <div className={`stepper-circle ${i < currentStep ? "done" : ""} ${i === currentStep ? "active" : ""}`}>
            {i < currentStep ? (
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M2 7.5L5.5 11L12 3" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            ) : (
              <span>{i + 1}</span>
            )}
          </div>
          <span className={`stepper-label ${i === currentStep ? "active" : ""}`}>{label}</span>
          {i < steps.length - 1 && <div className="stepper-line" />}
        </div>
      ))}
    </div>
  );
}
