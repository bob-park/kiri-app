import { useContext } from 'react';

import { I18nContext } from '@/shared/providers/I18nProvider';

export default function useI18n() {
  return useContext(I18nContext);
}
