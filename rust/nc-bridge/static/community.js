'use strict';
const notice = document.getElementById('notice');
async function request(path, method = 'GET') {
    const response = await fetch(path, {method, credentials: 'same-origin'});
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || 'Request failed');
    return data;
}
function button(parent, label, action) {
    const el = document.createElement('button'); el.textContent = label;
    el.onclick = async () => {el.disabled = true; try {await action();} catch (error) {notice.textContent = error.message;} finally {el.disabled = false;}};
    parent.append(el);
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
        if (game.state === 'recruiting' && !game.invite_required) button(card, 'Join', async () => {const seat = await request(`${base}/join`, 'POST'); notice.textContent = `Joined ${game.name}: seat ${seat.seat}. Click Play to enter.`; await loadGames();});
        if (game.invite_required && game.state === 'recruiting') {
            const invite = document.createElement('p'); invite.textContent = `Invite required. Get a private code from the host, then use /join game:${game.game} code:YOURCODE in Discord. Already joined? Click Play.`; card.append(invite);
        }
        button(card, 'Play', () => {location.href = `/terminal/index.html?game=${encodeURIComponent(game.game)}`;});
        button(card, 'Reconnect', async () => {await request(`${base}/reconnect`, 'POST'); notice.textContent = 'Previous session is closing. Click Play in a moment.';});
        for (const [label, format] of [['Map ZIP', 'map-zip'], ['Map TXT', 'map-txt'], ['Map CSV', 'map-csv'], ['Reports', 'reports']]) {
            const link = document.createElement('a'); link.textContent = ` ${label} `; link.href = `${base}/exports/${format}`; card.append(link);
        }
        container.append(card);
    }
    if (!games.length) container.textContent = 'No games are currently listed.';
}
document.getElementById('logout').onclick = async () => {try {await request('/auth/logout', 'POST'); notice.textContent = 'Signed out.';} catch (error) {notice.textContent = error.message;}};
loadGames().catch(error => {notice.textContent = error.message;});
