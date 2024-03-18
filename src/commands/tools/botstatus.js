const { SlashCommandBuilder, EmbedBuilder, ButtonBuilder, ActionRowBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('botstatus')
        .setDescription('Gets information about the bot'),
    async execute(interaction) {
        const bot = interaction.client.user;
        const botmem = await interaction.guild.members.fetch(bot.id);
        let totalSeconds = (interaction.client.uptime / 1000);
        let days = Math.floor(totalSeconds / 86400);
        totalSeconds %= 86400;
        let hours = Math.floor(totalSeconds / 3600);
        totalSeconds %= 3600;
        let minutes = Math.floor(totalSeconds / 60);
        let seconds = Math.floor(totalSeconds % 60);
        let uptime = `${days} days, ${hours} hours, ${minutes} minutes and ${seconds} seconds`;

        const row = new ActionRowBuilder()
            .addComponents(
                new ButtonBuilder()
                    .setLabel('Invite')
                    .setStyle('Link')
                    .setURL('https://discord.com/api/oauth2/authorize?client_id=1087418361283092510&permissions=8&scope=bot%20applications.commands'),
                new ButtonBuilder()
                    .setLabel('Code')
                    .setStyle('Link')
                    .setURL('https://github.com/isabelroses/blahaj')
            )

        const embed = new EmbedBuilder()
            .setTitle('Bot Status')
            .setColor([255, 255, 255])
            .setThumbnail(bot.displayAvatarURL({ dynamic: true }))
            .addFields({ name: 'Created At', value: `<t:${parseInt(bot.createdAt / 1000)}:R>`, inline: false })
            .addFields({ name: 'Joined At', value: `<t:${parseInt(botmem.joinedAt / 1000)}:R>`, inline: false })
            .addFields({ name: 'Ping', value: `${Math.round(interaction.client.ws.ping)}ms`, inline: false })
            .addFields({ name: 'Servers', value: `${interaction.client.guilds.cache.size}`, inline: false })
            .addFields({
                name: 'Uptime', value: `\`\`\`${uptime}\`\`\``, inline: true
            })
            .addFields({ name: 'Roles', value: `${botmem.roles.cache.map(r => r).join(' ')}`, inline: false })
            .setFooter({ text: "Bot ID: 1087418361283092510" })
            .setTimestamp(Date.now())
            .setAuthor({
                name: bot.tag,
                iconURL: bot.displayAvatarURL({ dynamic: true })
            });
        await interaction.reply({
            embeds: [embed],
            components: [row]
        })
    }
};