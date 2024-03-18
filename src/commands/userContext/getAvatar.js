const { ContextMenuCommandBuilder, ApplicationCommandType } = require('discord.js');

module.exports = {
    data: new ContextMenuCommandBuilder()
        .setName('Avatar')
        .setType(ApplicationCommandType.User),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        await interaction.reply({
            content: user.displayAvatarURL({ dynamic: true })
        });
    }
};
