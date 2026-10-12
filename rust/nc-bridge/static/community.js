'use strict';
const notice = document.getElementById('notice');
let currentAccount = null;
async function request(path, method = 'GET', body, headers = {}) {
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    const response = await fetch(path, {method, credentials: 'same-origin', headers, body});
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || 'Request failed');
    return data;
}
function button(parent, label, action) {
    const el = document.createElement('button'); el.textContent = label;
    el.onclick = async () => {el.disabled = true; try {await action();} catch (error) {notice.textContent = error.message;} finally {el.disabled = false;}};
    parent.append(el);
}
async function loadIdentity() {
    const response = await fetch('/api/me', {credentials: 'same-origin'});
    if (response.status === 401) {
        currentAccount = null;
        document.getElementById('identity').textContent = 'Choose a sign-in method to join or play.';
        return;
    }
    const me = await response.json();
    if (!response.ok) throw new Error(me.error || 'Cannot load account');
    currentAccount = me.account;
    const provider = me.account.provider === 'discord' ? 'Discord' : 'Nostr';
    const identifier = me.account.provider === 'nostr' ? `${me.account.id.slice(0,12)}…` : me.account.id;
    document.getElementById('identity').textContent = `Signed in as ${me.display_name} via ${provider} (${identifier}).`;
}
async function loadGames() {
    const games = await request('/api/games');
    const container = document.getElementById('games'); container.replaceChildren();
    for (const game of games) {
        const card = document.createElement('article');
        const title = document.createElement('h2'); title.textContent = game.name; card.append(title);
        const status = document.createElement('p');
        const schedule = game.schedule?.mode === 'manual' ? 'Manual turns' : game.schedule?.mode === 'daily_midnight' ? `Midnight ${game.schedule.time_zone} (daylight saving adjusted)` : `Every ${game.interval_minutes / 60} hours`;
        status.textContent = `${game.state} | ${game.open}/${game.players} seats open | Year ${game.year} | ${schedule}`;
        card.append(status);
        const due = document.createElement('p'); due.textContent = game.next_due ? `Next turn: ${new Date(game.next_due * 1000).toLocaleString()}` : 'Turns have not been scheduled.'; card.append(due);
        const base = `/api/games/${encodeURIComponent(game.game)}`;
        if (game.state === 'recruiting') {
            let code;
            if (game.invite_required) {
                const invite = document.createElement('p'); invite.textContent = 'Get a private invite code from the host. Already joined? Click Play using the same account.'; card.append(invite);
                const label = document.createElement('label'); label.textContent = 'Invite code ';
                code = document.createElement('input'); code.type = 'password'; code.maxLength = 8; code.autocomplete = 'off'; code.spellcheck = false;
                label.append(code); card.append(label);
            }
            button(card, 'Join', async () => {
                if (!currentAccount) throw new Error('Sign in with Discord or Nostr first.');
                const body = code ? {code:code.value.trim()} : {};
                const seat = await request(`${base}/join`, 'POST', JSON.stringify(body));
                notice.textContent = `Joined ${game.name}: seat ${seat.seat}. Click Play to enter.`;
                await loadGames();
            });
        }
        button(card, 'Play', () => {location.href = `/terminal/index.html?game=${encodeURIComponent(game.game)}`;});
        button(card, 'Reconnect', async () => {await request(`${base}/reconnect`, 'POST'); notice.textContent = 'Previous session is closing. Click Play in a moment.';});
        button(card, 'Download map ZIP', () => {location.href = `${base}/exports/map-zip`;});
        container.append(card);
    }
    if (!games.length) container.textContent = 'No games are currently listed.';
}
document.getElementById('nostr-login').onsubmit = async event => {
    event.preventDefault();
    const submit = event.currentTarget.querySelector('button'); submit.disabled = true;
    try {
        if (!window.nostr?.signEvent) throw new Error('Install or enable a NIP-07 extension such as nos2x, then reload this page.');
        notice.textContent = 'Approve the login request in your Nostr extension.';
        const challenge = await request('/auth/nostr/challenge', 'POST');
        const body = JSON.stringify({challenge:challenge.challenge, display_name:document.getElementById('nostr-name').value.trim() || null});
        const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(body));
        const payload = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2,'0')).join('');
        const signed = await window.nostr.signEvent({kind:27235, created_at:Math.floor(Date.now()/1000), content:'', tags:[['u',`${location.origin}/auth/nostr`],['method','POST'],['payload',payload]]});
        await request('/auth/nostr', 'POST', body, {Authorization:`Nostr ${btoa(JSON.stringify(signed))}`});
        await loadIdentity(); await loadGames();
        notice.textContent = 'Signed in. Use your invite code to join, or Play to return to your seat.';
    } catch (error) {
        notice.textContent = error?.message || 'Nostr login was cancelled or failed. Try again when ready.';
    } finally {submit.disabled = false;}
};
document.getElementById('logout').onclick = async () => {try {await request('/auth/logout', 'POST'); await loadIdentity(); await loadGames(); notice.textContent = 'Signed out.';} catch (error) {notice.textContent = error.message;}};
(async () => {await loadIdentity(); await loadGames();})().catch(error => {notice.textContent = error.message;});
