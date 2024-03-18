const { ActivityType } = require('discord.js');

module.exports = (client) => {
    client.pickPresence = async () => {
        const options = [
            { text: 'with Slash Commands', type: ActivityType.Playing, url: null, status: 'online' },
            { text: '/help', type: ActivityType.Watching, url: null, status: 'online' },
        ];
        const option = Math.floor(Math.random() * options.length);

        client.user.setPresence({
            activities: [{
                name: options[option].text,
                type: options[option].type,
                url: options[option].url,
            }],
            status: options[option].status,
        });
        console.log(`Presence set to ${options[option].text}`);
    }
}
