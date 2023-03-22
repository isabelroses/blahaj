const { ContextMenuCommandBuilder } = require('@discordjs/builders');
const { ApplicationCommandType } = require('discord-api-types/v9');

module.exports = {
    data: new ContextMenuCommandBuilder()
        .setName('avatar')
        .setType(ApplicationCommandType.User),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        await interaction.reply({
            content: user.displayAvatarURL({ dynamic: true })
        });
    }
};
