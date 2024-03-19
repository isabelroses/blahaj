const { SlashCommandBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('avatar')
        .setDescription('Replies with your avatar!')
        .addUserOption(option => option.setName('target').setDescription('The user\'s avatar to show').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('target');
        if (!user) {
            await interaction.reply({ content: `${interaction.user.displayAvatarURL({ dynamic: true })}` });
        } else {
            await interaction.reply({ content: `${interaction.options.getUser('target').displayAvatarURL({ dynamic: true })}` });
        }
    },
};
