const { SlashCommandBuilder, EmbedBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('whois')
        .setDescription('Gets information about a user')
        .addUserOption(option => option.setName('user').setDescription('The user to get information about').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('user') || interaction.user;
        const embed = new EmbedBuilder()
            .setTitle(`${user.username}#${user.discriminator}`)
            .setDescription(`ID: ${user.id}`)
            .setColor([255, 255, 255])
            .setThumbnail(user.displayAvatarURL({ dynamic: true }))
            .addField('Created At', user.createdAt.toUTCString(), true)
            .addField('Joined At', interaction.guild.members.cache.get(user.id).joinedAt.toUTCString(), true)
            .addField('Bot', user.bot, true)
            .addField('Status', user.presence.status, true)
            .addField('Activity', user.presence.activities[0] ? user.presence.activities[0].name : 'None', true)
            .addField('Roles', interaction.guild.members.cache.get(user.id).roles.cache.map(role => role.toString()).join(' '), true)
            .setFooter({
                iconURL: interaction.client.user.displayAvatarURL({ dynamic: true }),
                text: interaction.client.user.tag
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
