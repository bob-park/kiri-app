import type { AppError } from '@/shared/api/common.dto';

export interface ErrorState {
  /** The last failed action, shown by `ErrorAlert` until dismissed. */
  error?: AppError;
  showError: (error: AppError) => void;
  dismissError: () => void;
}
