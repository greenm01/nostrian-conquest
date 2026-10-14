'use strict';
const notice = document.getElementById('notice');
let currentAccount = null;
let gamesLoad = 0;
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
        document.getElementById('nostr-login').hidden = false;
        document.getElementById('logout').hidden = true;
        document.getElementById('identity').textContent = 'Choose a sign-in method to join or play.';
        return;
    }
    const me = await response.json();
    if (!response.ok) throw new Error(me.error || 'Cannot load account');
    currentAccount = me.account;
    document.getElementById('nostr-login').hidden = true;
    document.getElementById('matrix-login').hidden = true;
    document.getElementById('logout').hidden = false;
    const provider = {discord:'Discord', nostr:'Nostr', matrix:'Matrix'}[me.account.provider];
    const identifier = me.account.provider === 'nostr' ? `${me.account.id.slice(0,12)}…` : me.account.id;
    document.getElementById('identity').textContent = `Signed in as ${me.display_name} via ${provider} (${identifier}).`;
}
async function loadGames() {
    const revision = ++gamesLoad;
    const games = await request('/api/games');
    if (revision !== gamesLoad) return;
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
        container.append(card);
        if (!currentAccount) {
            const prompt = document.createElement('p'); prompt.textContent = 'Sign in to join or play.'; card.append(prompt);
            continue;
        }
        let access;
        try {
            access = await request(`${base}/access`);
            if (revision !== gamesLoad) return;
        } catch (error) {
            const message = document.createElement('p'); message.textContent = `Cannot check your seat: ${error.message}`; card.append(message);
            continue;
        }
        if (access.seat !== null) {
            const seat = document.createElement('p'); seat.textContent = `Your seat: ${access.seat}`; card.append(seat);
            if (access.can_play) {
                if (access.connected) {
                    const message = document.createElement('p'); message.textContent = 'Your seat is open in another terminal. Reconnect to close it and continue here.'; card.append(message);
                    button(card, 'Reconnect', async () => {
                        await request(`${base}/reconnect`, 'POST');
                        notice.textContent = 'Closing the previous terminal…';
                        for (let attempt = 0; attempt < 20; attempt++) {
                            await new Promise(resolve => setTimeout(resolve, 250));
                            const latest = await request(`${base}/access`);
                            if (!latest.connected) {
                                await loadGames();
                                notice.textContent = 'Previous terminal closed. Click Play to continue.';
                                return;
                            }
                        }
                        throw new Error('Previous terminal is still closing. Try Reconnect again shortly.');
                    });
                } else {
                    button(card, 'Play', () => {location.href = `/terminal/index.html?game=${encodeURIComponent(game.game)}`;});
                }
            } else {
                const message = document.createElement('p');
                message.textContent = access.state === 'paused' ? 'Paused by host.' : access.state === 'maintenance' ? 'Turn processing is in progress.' : 'This game is not accepting play.';
                card.append(message);
            }
            button(card, 'Download map ZIP', () => {location.href = `${base}/exports/map-zip`;});
        } else if (access.state === 'recruiting' && game.open > 0) {
            let code;
            if (game.invite_required) {
                const invite = document.createElement('p'); invite.textContent = 'Get a private invite code from the host.'; card.append(invite);
                const label = document.createElement('label'); label.textContent = 'Invite code ';
                code = document.createElement('input'); code.type = 'password'; code.maxLength = 8; code.autocomplete = 'off'; code.spellcheck = false;
                label.append(code); card.append(label);
            }
            button(card, 'Join', async () => {
                const body = code ? {code:code.value.trim()} : {};
                const seat = await request(`${base}/join`, 'POST', JSON.stringify(body));
                notice.textContent = `Joined ${game.name}: seat ${seat.seat}. Click Play to enter.`;
                await loadGames();
            });
        } else {
            const message = document.createElement('p');
            message.textContent = access.state === 'recruiting' ? 'No seats available.' : 'Recruitment is closed.';
            card.append(message);
        }
    }
    if (!games.length) container.textContent = 'No games are currently listed.';
}
document.getElementById('nostr-login').onsubmit = async event => {
    event.preventDefault();
    const submit = event.currentTarget.querySelector('button'); submit.disabled = true;
    try {
        if (!window.nostr?.getPublicKey || !window.nostr?.signEvent) throw new Error('Install or enable a NIP-07 extension such as nos2x, then reload this page.');
        notice.textContent = 'Approve the login request in your Nostr extension.';
        const pubkey = await window.nostr.getPublicKey();
        if (typeof pubkey !== 'string' || !/^[0-9a-f]{64}$/.test(pubkey)) throw new Error('Your Nostr extension did not provide a public key. Set up or unlock your account in the extension, then try again.');
        const challenge = await request('/auth/nostr/challenge', 'POST');
        const body = JSON.stringify({challenge:challenge.challenge});
        const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(body));
        const payload = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2,'0')).join('');
        const signed = await window.nostr.signEvent({pubkey, kind:27235, created_at:Math.floor(Date.now()/1000), content:'', tags:[['u',`${location.origin}/auth/nostr`],['method','POST'],['payload',payload]]});
        await request('/auth/nostr', 'POST', body, {Authorization:`Nostr ${btoa(JSON.stringify(signed))}`});
        await loadIdentity(); await loadGames();
        notice.textContent = 'Signed in.';
    } catch (error) {
        notice.textContent = error?.message || 'Nostr login was cancelled or failed. Try again when ready.';
    } finally {submit.disabled = false;}
};
async function matrixSignIn(code) {
    await request('/auth/matrix', 'POST', JSON.stringify({code}));
    await loadIdentity(); await loadGames();
    notice.textContent = 'Signed in.';
}
document.getElementById('matrix-start').onclick = () => {
    document.getElementById('matrix-login').hidden = false;
    document.getElementById('matrix-id').focus();
};
document.getElementById('matrix-login').onsubmit = async event => {
    event.preventDefault();
    const submit = event.currentTarget.querySelector('button[type=submit]'); submit.disabled = true;
    try {await matrixSignIn(document.getElementById('matrix-code').value.trim());} catch (error) {notice.textContent = error.message;} finally {submit.disabled = false;}
};
document.getElementById('matrix-send').onclick = async event => {
    const send = event.currentTarget; send.disabled = true;
    try {
        await request('/auth/matrix/request', 'POST', JSON.stringify({matrix_id:document.getElementById('matrix-id').value.trim()}));
        notice.textContent = 'Check Matrix. Accept the invitation from Nostrian Conquest if asked, then open the link or enter the code here.';
        document.getElementById('matrix-code').focus();
    } catch (error) {notice.textContent = error.message;} finally {send.disabled = false;}
};
document.getElementById('logout').onclick = async () => {try {await request('/auth/logout', 'POST'); await loadIdentity(); await loadGames(); notice.textContent = 'Signed out.';} catch (error) {notice.textContent = error.message;}};
window.addEventListener('pageshow', () => {
    (async () => {
        // A Matrix sign-in link carries its secret in the fragment, which never reaches a server.
        const link = location.hash.match(/^#matrix-login=([0-9a-f]{64})$/);
        if (link) {
            history.replaceState(null, '', location.pathname + location.search);
            try {await matrixSignIn(link[1]); return;} catch (error) {notice.textContent = error.message;}
        }
        await loadIdentity(); await loadGames();
    })().catch(error => {notice.textContent = error.message;});
});
