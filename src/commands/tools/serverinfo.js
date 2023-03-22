const { SlashCommandBuilder, EmbedBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('serverinfo')
        .setDescription('Replies with server info!'),
    async execute(interaction) {

        const { guild } = interaction;
        const { name, ownerId, createdAt, region, memberCount } = guild;
        const { icon } = guild.iconURL({ dynamic: true });
        const { roles } = guild.roles.cache.size;
        const { emojies } = guild.emojis.cache.size;
        const { id } = guild.id;

        let baseVerificationLevel = guild.verificationLevel;

        if (baseVerificationLevel === '0') baseVerificationLevel = 'None';
        if (baseVerificationLevel === '1') baseVerificationLevel = 'Low';
        if (baseVerificationLevel === '2') baseVerificationLevel = 'Medium';
        if (baseVerificationLevel === '3') baseVerificationLevel = 'High';
        if (baseVerificationLevel === '4') baseVerificationLevel = 'Very High';

        const embed = new EmbedBuilder()
            .setTitle(`Server Info for ${name}`)
            .setThumbnail(icon)
            .addFields({ Name: 'Server Name', value: `${name}`, inline: true })
            .addFields({ Name: 'Server ID', value: `${id}`, inline: true })
            .addFields({ Name: 'Server Owner', value: `<@${ownerId}>`, inline: true })
            .addFields({ Name: 'Server Region', value: `${region}`, inline: true })
            .addFields({ Name: 'Server Created At', value: `${createdAt}`, inline: true })
            .addFields({ Name: 'Server Member Count', value: `${memberCount}`, inline: true })
            .addFields({ Name: 'Server Verification Level', value: `${baseVerificationLevel}`, inline: true })
            .addFields({ Name: 'Server Roles', value: `${roles}`, inline: true })
            .addFields({ Name: 'Server Emojis', value: `${emojies}`, inline: true })
            .addFields({ Name: 'Server Boosts', value: `${guild.premiumSubscriptionCount}`, inline: true })
            .setColor([255, 255, 255])
            .setFooter(`Requested by ${interaction.user.tag}`, interaction.user.avatarURL({ dynamic: true }))
            .setTimestamp(Date.now())
            .setAuthor(`${interaction.user.tag}`, interaction.user.avatarURL({ dynamic: true }));

        await interaction.reply({ embeds: [embed] });
    },
};
