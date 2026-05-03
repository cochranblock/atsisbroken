// SPDX-License-Identifier: Unlicense
// content.js — runs in every page. Watches form fields. On blur (user
// finished typing), records a FieldDescriptor + the predicted/observed
// keys into chrome.storage.local. Background.js later drains the queue
// to the native binary via Native Messaging.
//
// Intentionally NEVER stores the user's typed value. Only the field
// shape and the matched-or-not boolean. PII stays in the page.

(function () {
  const STORAGE_KEY = 'atsisbroken_observations';

  function describe(el) {
    const labelEl =
      (el.id && document.querySelector(`label[for="${el.id}"]`)) ||
      el.closest('label');
    return {
      label: labelEl ? labelEl.innerText.trim() : '',
      placeholder: el.placeholder || '',
      aria_label: el.getAttribute('aria-label') || '',
      name: el.name || '',
      id: el.id || '',
      kind: el.type || el.tagName.toLowerCase(),
    };
  }

  // Cheap keyword pre-classifier. Same vocabulary as the seed corpus.
  // Server-side classifier overrides this once the bridge delivers.
  function predictKey(field) {
    const hay = `${field.label} ${field.placeholder} ${field.aria_label} ${field.name} ${field.id}`.toLowerCase();
    if (hay.includes('email')) return 'email';
    if (hay.includes('phone') || hay.includes('mobile') || hay.includes('tel')) return 'phone';
    if (hay.includes('linkedin')) return 'linkedin';
    if (hay.includes('github')) return 'github';
    if (hay.includes('website') || hay.includes('portfolio')) return 'website';
    if (hay.includes('address') || hay.includes('street') || hay.includes('city')) return 'address';
    if (hay.includes('authoriz') || hay.includes('visa') || hay.includes('sponsor')) return 'work_authorization';
    if ((hay.includes('year') || hay.includes('yrs')) && hay.includes('exp')) return 'years_experience';
    if (hay.includes('name')) return 'full_name';
    if (field.kind === 'textarea') return 'freetext';
    return 'unknown';
  }

  function recordObservation(el) {
    if (!el.value || !el.value.trim()) return; // user typed nothing
    const field = describe(el);
    const predicted = predictKey(field);
    const observation = {
      field,
      predicted,
      observed: predicted, // refined by the trained classifier server-side
      confidence: 0.5, // placeholder — server overwrites
    };
    chrome.storage.local.get([STORAGE_KEY], (data) => {
      const queue = data[STORAGE_KEY] || [];
      queue.push(observation);
      chrome.storage.local.set({ [STORAGE_KEY]: queue });
    });
  }

  function attach() {
    document
      .querySelectorAll('input, textarea, select')
      .forEach((el) =>
        el.addEventListener('blur', () => recordObservation(el), { passive: true }),
      );
  }

  attach();
  // Re-attach on DOM mutation (Workday-style late-rendered forms).
  new MutationObserver(attach).observe(document.body, {
    childList: true,
    subtree: true,
  });
})();
