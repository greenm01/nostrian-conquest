'use strict';
for (const button of document.querySelectorAll('[data-copy]')) {
    button.onclick = async () => {
        const text = document.getElementById(button.dataset.copy).textContent;
        const status = document.getElementById('copy-status');
        try {
            await navigator.clipboard.writeText(text);
            status.textContent = button.dataset.copy === 'nostr-contact' ? 'Public key copied.' : 'Username copied.';
        } catch (_) {
            status.textContent = 'Could not copy. Select the text above and copy it.';
        }
    };
}
