import { writable } from 'svelte/store';

export const showBillingPlanModal = writable(false);

// When true, closing the billing modal should reopen the settings modal
export const reopenSettingsAfterBilling = writable(false);
