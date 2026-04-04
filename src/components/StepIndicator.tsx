interface StepIndicatorProps {
  steps: string[];
  currentStep: number;
}

export default function StepIndicator({ steps, currentStep }: StepIndicatorProps) {
  return (
    <nav className="step-nav" aria-label="步骤导航">
      {steps.map((label, i) => {
        const isActive = i === currentStep;
        const isDone = i < currentStep;
        const dotClass = isDone ? "step-dot--done" : isActive ? "step-dot--active" : "step-dot--pending";
        const labelClass = isDone ? "step-label--done" : isActive ? "step-label--active" : "step-label--pending";

        return (
          <div key={label} className="flex items-center" {...(isActive ? { "aria-current": "step" } : {})}>
            <div className="step-node">
              <span className={`step-dot ${dotClass}`}>
                {isDone ? (
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                    <path d="M2 6.5L4.5 9L10 3" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                ) : (
                  <span>{i + 1}</span>
                )}
              </span>
              <span className={`step-label ${labelClass}`}>{label}</span>
            </div>
            {i < steps.length - 1 && (
              <div className={`step-connector ${i < currentStep ? "step-connector--done" : "step-connector--pending"}`} />
            )}
          </div>
        );
      })}
    </nav>
  );
}
