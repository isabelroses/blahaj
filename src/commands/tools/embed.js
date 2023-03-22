const { SlashCommandBuilder, EmbedBuilder, Embed } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('embed')
        .setDescription('Sends an embed')
        .addStringOption(option => option.setName('title').setDescription('The title of the embed').setRequired(true))
        .addStringOption(option => option.setName('description').setDescription('The description of the embed').setRequired(true))
        .addStringOption(option => option.setName('image').setDescription('Image').setRequired(false)),
    async execute(interaction, client) {
        const embed = new EmbedBuilder()
            .setTitle(interaction.options.getString('title'))
            .setDescription(interaction.options.getString('description'))
            .setThumbnail(interaction.options.getString('image') || null)
            .setColor([255, 255, 255])
            .setThumbnail(interaction.guild.iconURL({ dynamic: true }))
            .setFooter({
                iconURL: client.user.displayAvatarURL({ dynamic: true }),
                text: client.user.tag
            })
            .setTimestamp(Date.now())
            .setAuthor({
                name: interaction.user.tag,
                iconURL: interaction.user.displayAvatarURL({ dynamic: true })
            });
        await interaction.reply({
            embeds: [embed]
        })
    }
};