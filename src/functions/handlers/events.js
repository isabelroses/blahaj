const path = require('path');
const fs = require('fs');

module.exports = (client) => {
    client.handleEvents = async () => {
        const eventFolderPath = path.join(__dirname, '../', '../', 'events');
        const eventFolders = fs.readdirSync(eventFolderPath);

        for (const folder of eventFolders) {
            const folderPath = path.join(eventFolderPath, folder);
            const eventFiles = fs.readdirSync(folderPath).filter(file => file.endsWith('.js'));

            switch (folder) {
                case "client":
                    for (const file of eventFiles) {
                        const event = require(path.join(folderPath, file));
                        if (event.once) client.once(event.name, (...args) => event.execute(...args, client));
                        else client.on(event.name, (...args) => event.execute(...args, client));
                    }
                    break;
                default:
                    break;
            }
        }
    };
};

